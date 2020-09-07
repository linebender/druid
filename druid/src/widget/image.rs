// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! An Image widget.
//! Please consider using SVG and the SVG widget as it scales much better.

use std::fmt;
#[cfg(feature = "image")]
use std::{convert::AsRef, error::Error, path::Path};

use crate::{
    piet::{Image as PietImage, ImageFormat, InterpolationMode},
    widget::common::FillStrat,
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Rect,
    RenderContext, Size, UpdateCtx, Widget,
};

/// A widget that renders a bitmap Image.
///
/// Contains data about how to fill the given space and interpolate pixels.
/// Configuration options are provided via the builder pattern.
///
/// Note: when [scaling a bitmap image], such as supporting multiple
/// screen sizes and resolutions, interpolation can lead to blurry
/// or pixelated images and so is not recommended for things like icons.
/// Instead consider using [SVG files] and enabling the `svg` feature with `cargo`.
///
/// (See also:
/// [`ImageData`],
/// [`FillStrat`],
/// [`InterpolationMode`]
/// )
///
/// # Example
///
/// Create an image widget and configure it using builder methods
/// ```
/// use druid::{
///     widget::{Image, ImageData, FillStrat},
///     piet::InterpolationMode,
/// };
///
/// let image_data = ImageData::empty();
/// let image_widget = Image::new(image_data)
///     // set the fill strategy
///     .fill_mode(FillStrat::Fill)
///     // set the interpolation mode
///     .interpolation_mode(InterpolationMode::Bilinear);
/// ```
/// Create an image widget and configure it using setters
/// ```
/// use druid::{
///     widget::{Image, ImageData, FillStrat},
///     piet::InterpolationMode,
/// };
///
/// let image_data = ImageData::empty();
/// let mut image_widget = Image::new(image_data);
/// // set the fill strategy
/// image_widget.set_fill_mode(FillStrat::FitWidth);
/// // set the interpolation mode
/// image_widget.set_interpolation_mode(InterpolationMode::Bilinear);
/// ```
///
/// [scaling a bitmap image]: ../struct.Scale.html#pixels-and-display-points
/// [SVG files]: https://en.wikipedia.org/wiki/Scalable_Vector_Graphics
/// [`ImageData`]: struct.ImageData.html
/// [`FillStrat`]: ../widget/enum.FillStrat.html
/// [`InterpolationMode`]: ../piet/enum.InterpolationMode.html
pub struct Image {
    image_data: ImageData,
    paint_data: Option<PietImage>,
    fill: FillStrat,
    interpolation: InterpolationMode,
}

impl Image {
    /// Create an image drawing widget from `ImageData`.
    ///
    /// By default, the Image will scale to fit its box constraints
    /// ([`FillStrat::Fill`])
    /// and will be scaled bilinearly
    /// ([`InterpolationMode::Bilinear`])
    ///
    /// [`FillStrat::Fill`]: ../widget/enum.FillStrat.html#variant.Fill
    /// [`InterpolationMode::Bilinear`]: ../piet/enum.InterpolationMode.html#variant.Bilinear
    pub fn new(image_data: ImageData) -> Self {
        Image {
            image_data,
            paint_data: None,
            fill: FillStrat::default(),
            interpolation: InterpolationMode::Bilinear,
        }
    }

    /// A builder-style method for specifying the fill strategy.
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        self
    }

    /// Modify the widget's fill strategy.
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.fill = newfil;
        self.paint_data = None;
    }

    /// A builder-style method for specifying the interpolation strategy.
    pub fn interpolation_mode(mut self, interpolation: InterpolationMode) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Modify the widget's interpolation mode.
    pub fn set_interpolation_mode(&mut self, interpolation: InterpolationMode) {
        self.interpolation = interpolation;
        self.paint_data = None;
    }
}

impl<T: Data> Widget<T> for Image {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &T, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.debug_check("Image");

        if bc.is_width_bounded() {
            bc.max()
        } else {
            bc.constrain(self.image_data.get_size())
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let offset_matrix = self
            .fill
            .affine_to_fill(ctx.size(), self.image_data.get_size());

        // The ImageData's to_piet function does not clip to the image's size
        // CairoRenderContext is very like druids but with some extra goodies like clip
        if self.fill != FillStrat::Contain {
            let clip_rect = Rect::ZERO.with_size(ctx.size());
            ctx.clip(clip_rect);
        }

        ctx.with_save(|ctx| {
            let piet_image = {
                let image_data = &self.image_data;
                self.paint_data
                    .get_or_insert_with(|| image_data.to_piet(ctx))
            };
            ctx.transform(offset_matrix);
            ctx.draw_image(
                piet_image,
                self.image_data.get_size().to_rect(),
                self.interpolation,
            );
        });
    }
}

/// Processed image data.
///
/// By default, Druid does not parse image data.
/// However, enabling [the `image` feature]
/// provides several
/// methods by which you can load image files.
///
/// Contains raw bytes, dimensions, and image format ([`piet::ImageFormat`]).
///
/// [the `image` feature]: ../index.html#optional-features
/// [`piet::ImageFormat`]: ../piet/enum.ImageFormat.html
#[derive(Clone)]
pub struct ImageData {
    pixels: Vec<u8>,
    x_pixels: u32,
    y_pixels: u32,
    format: ImageFormat,
}

impl ImageData {
    /// Create an empty Image
    pub fn empty() -> Self {
        ImageData {
            pixels: [].to_vec(),
            x_pixels: 0,
            y_pixels: 0,
            format: ImageFormat::RgbaSeparate,
        }
    }

    /// Get the size in pixels of the contained image.
    fn get_size(&self) -> Size {
        Size::new(self.x_pixels as f64, self.y_pixels as f64)
    }

    /// Convert ImageData into Piet draw instructions.
    fn to_piet(&self, ctx: &mut PaintCtx) -> PietImage {
        ctx.make_image(
            self.get_size().width as usize,
            self.get_size().height as usize,
            &self.pixels,
            self.format,
        )
        .unwrap()
    }
}

#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
impl ImageData {
    /// Load an image from a DynamicImage from the image crate
    pub fn from_dynamic_image(image_data: image::DynamicImage) -> ImageData {
        use image::ColorType::*;
        let has_alpha_channel = match image_data.color() {
            La8 | Rgba8 | La16 | Rgba16 | Bgra8 => true,
            _ => false,
        };

        if has_alpha_channel {
            Self::from_dynamic_image_with_alpha(image_data)
        } else {
            Self::from_dynamic_image_without_alpha(image_data)
        }
    }

    /// Load an image from a DynamicImage with alpha
    pub fn from_dynamic_image_with_alpha(image_data: image::DynamicImage) -> ImageData {
        let rgba_image = image_data.to_rgba();
        let sizeofimage = rgba_image.dimensions();
        ImageData {
            pixels: rgba_image.to_vec(),
            x_pixels: sizeofimage.0,
            y_pixels: sizeofimage.1,
            format: ImageFormat::RgbaSeparate,
        }
    }

    /// Load an image from a DynamicImage without alpha
    pub fn from_dynamic_image_without_alpha(image_data: image::DynamicImage) -> ImageData {
        let rgb_image = image_data.to_rgb();
        let sizeofimage = rgb_image.dimensions();
        ImageData {
            pixels: rgb_image.to_vec(),
            x_pixels: sizeofimage.0,
            y_pixels: sizeofimage.1,
            format: ImageFormat::Rgb,
        }
    }

    /// Attempt to load an image from raw bytes.
    ///
    /// If the image crate can't decode an image from the data an error will be returned.
    pub fn from_data(raw_image: &[u8]) -> Result<Self, Box<dyn Error>> {
        let image_data = image::load_from_memory(raw_image).map_err(|e| e)?;
        Ok(ImageData::from_dynamic_image(image_data))
    }

    /// Attempt to load an image from the file at the provided path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let image_data = image::open(path).map_err(|e| e)?;
        Ok(ImageData::from_dynamic_image(image_data))
    }
}

impl Default for ImageData {
    fn default() -> Self {
        ImageData::empty()
    }
}

impl fmt::Debug for ImageData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImageData")
            .field("size", &self.pixels.len())
            .field("width", &self.x_pixels)
            .field("height", &self.y_pixels)
            .field("format", &format_args!("{:?}", self.format))
            .finish()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tall_paint() {
        use crate::{tests::harness::Harness, WidgetId};

        let _id_1 = WidgetId::next();
        let image_data = ImageData {
            pixels: vec![255, 255, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255],
            x_pixels: 2,
            y_pixels: 2,
            format: ImageFormat::Rgb,
        };

        let image_widget =
            Image::new(image_data).interpolation_mode(InterpolationMode::NearestNeighbor);

        Harness::create_with_render(
            true,
            image_widget,
            Size::new(400., 600.),
            |harness| {
                harness.send_initial_events();
                harness.just_layout();
                harness.paint();
            },
            |target| {
                let raw_pixels = target.into_raw();
                assert_eq!(raw_pixels.len(), 400 * 600 * 4);

                // Being a tall widget with a square image the top and bottom rows will be
                // the padding color and the middle rows will not have any padding.

                // Check that the middle row 400 pix wide is 200 black then 200 white.
                let expecting: Vec<u8> = [
                    vec![0, 0, 0, 255].repeat(200),
                    vec![255, 255, 255, 255].repeat(200),
                ]
                .concat();
                assert_eq!(raw_pixels[400 * 300 * 4..400 * 301 * 4], expecting[..]);

                // Check that all of the last 100 rows are all the background color.
                let expecting: Vec<u8> = vec![41, 41, 41, 255].repeat(400 * 100);
                assert_eq!(
                    raw_pixels[400 * 600 * 4 - 4 * 400 * 100..400 * 600 * 4],
                    expecting[..]
                );
            },
        )
    }
    #[test]
    fn wide_paint() {
        use crate::{tests::harness::Harness, WidgetId};
        let _id_1 = WidgetId::next();
        let image_data = ImageData {
            pixels: vec![255, 255, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255],
            x_pixels: 2,
            y_pixels: 2,
            format: ImageFormat::Rgb,
        };

        let image_widget =
            Image::new(image_data).interpolation_mode(InterpolationMode::NearestNeighbor);

        Harness::create_with_render(
            true,
            image_widget,
            Size::new(600., 400.),
            |harness| {
                harness.send_initial_events();
                harness.just_layout();
                harness.paint();
            },
            |target| {
                let raw_pixels = target.into_raw();
                assert_eq!(raw_pixels.len(), 400 * 600 * 4);

                // Being a wide widget every row will have some padding at the start and end
                // the last row will be like this too and there will be no padding rows at the end.

                // A middle row of 600 pixels is 100 padding 200 black, 200 white and then 100 padding.
                let expecting: Vec<u8> = [
                    vec![41, 41, 41, 255].repeat(100),
                    vec![255, 255, 255, 255].repeat(200),
                    vec![0, 0, 0, 255].repeat(200),
                    vec![41, 41, 41, 255].repeat(100),
                ]
                .concat();
                assert_eq!(raw_pixels[199 * 600 * 4..200 * 600 * 4], expecting[..]);

                // The final row of 600 pixels is 100 padding 200 black, 200 white and then 100 padding.
                let expecting: Vec<u8> = [
                    vec![41, 41, 41, 255].repeat(100),
                    vec![0, 0, 0, 255].repeat(200),
                    vec![255, 255, 255, 255].repeat(200),
                    vec![41, 41, 41, 255].repeat(100),
                ]
                .concat();
                assert_eq!(raw_pixels[399 * 600 * 4..400 * 600 * 4], expecting[..]);
            },
        );
    }
    #[test]
    fn into_png() {
        use crate::{
            tests::{harness::Harness, temp_dir_for_test},
            WidgetId,
        };
        let _id_1 = WidgetId::next();
        let image_data = ImageData {
            pixels: vec![255, 255, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255],
            x_pixels: 2,
            y_pixels: 2,
            format: ImageFormat::Rgb,
        };

        let image_widget =
            Image::new(image_data).interpolation_mode(InterpolationMode::NearestNeighbor);

        Harness::create_with_render(
            true,
            image_widget,
            Size::new(600., 400.),
            |harness| {
                harness.send_initial_events();
                harness.just_layout();
                harness.paint();
            },
            |target| {
                let tmp_dir = temp_dir_for_test();
                target.into_png(tmp_dir.join("image.png")).unwrap();
            },
        );
    }
}
