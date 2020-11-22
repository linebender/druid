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

#[cfg(feature = "image")]
use std::error::Error;
#[cfg(feature = "image")]
use std::path::Path;
use std::sync::Arc;

use kurbo::Size;
use piet_common::{Color, ImageFormat, RenderContext};

/// An in-memory pixel buffer.
///
/// Contains raw bytes, dimensions, and image format ([`piet::ImageFormat`]).
///
/// [`piet::ImageFormat`]: ../piet/enum.ImageFormat.html
#[derive(Clone)]
pub struct ImageBuf {
    pixels: Arc<[u8]>,
    width: usize,
    height: usize,
    format: ImageFormat,
}

impl ImageBuf {
    /// Create an empty image buffer.
    pub fn empty() -> Self {
        ImageBuf {
            pixels: Arc::new([]),
            width: 0,
            height: 0,
            format: ImageFormat::RgbaSeparate,
        }
    }

    /// Creates a new image buffer from an array of bytes.
    ///
    /// `format` specifies the pixel format of the pixel data, which must have length
    /// `width * height * format.bytes_per_pixel()`.
    ///
    /// # Panics
    ///
    /// Panics if the pixel data has the wrong length.
    pub fn from_raw(
        pixels: impl Into<Arc<[u8]>>,
        format: ImageFormat,
        width: usize,
        height: usize,
    ) -> ImageBuf {
        let pixels = pixels.into();
        assert_eq!(pixels.len(), width * height * format.bytes_per_pixel());
        ImageBuf {
            pixels,
            format,
            width,
            height,
        }
    }

    /// Returns the raw pixel data of this image buffer.
    pub fn raw_pixels(&self) -> &[u8] {
        &self.pixels[..]
    }

    /// Returns a shared reference to the raw pixel data of this image buffer.
    pub fn raw_pixels_shared(&self) -> Arc<[u8]> {
        Arc::clone(&self.pixels)
    }

    /// Returns the format of the raw pixel data.
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// The width, in pixels, of this image.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The height, in pixels, of this image.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The size of this image, in pixels.
    pub fn size(&self) -> Size {
        Size::new(self.width() as f64, self.height() as f64)
    }

    /// Returns an iterator over the pixels in this image.
    ///
    /// The return value is an iterator over "rows", where each "row" is an iterator
    /// over the color of the pixels in that row.
    pub fn pixel_colors<'a>(
        &'a self,
    ) -> impl Iterator<Item = impl Iterator<Item = Color> + 'a> + 'a {
        // TODO: a version of this exists in piet-web and piet-coregraphics. Maybe put it somewhere
        // common?
        fn unpremul(x: u8, a: u8) -> u8 {
            if a == 0 {
                0
            } else {
                let y = (x as u32 * 255 + (a as u32 / 2)) / (a as u32);
                y.min(255) as u8
            }
        }
        let format = self.format;
        let bytes_per_pixel = format.bytes_per_pixel();
        self.pixels
            .chunks_exact(self.width * bytes_per_pixel)
            .map(move |row| {
                row.chunks_exact(bytes_per_pixel)
                    .map(move |p| match format {
                        ImageFormat::Rgb => Color::rgb8(p[0], p[1], p[2]),
                        ImageFormat::RgbaSeparate => Color::rgba8(p[0], p[1], p[2], p[3]),
                        ImageFormat::RgbaPremul => {
                            let a = p[3];
                            Color::rgba8(unpremul(p[0], a), unpremul(p[1], a), unpremul(p[2], a), a)
                        }
                        // TODO: is there a better way to handle unsupported formats?
                        _ => Color::WHITE,
                    })
            })
    }

    /// Converts this buffer a Piet image, which is optimized for drawing into a [`RenderContext`].
    ///
    /// [`RenderContext`]: ../piet/trait.RenderContext.html
    pub fn to_piet_image<Ctx: RenderContext>(&self, ctx: &mut Ctx) -> Ctx::Image {
        ctx.make_image(self.width(), self.height(), &self.pixels, self.format)
            .unwrap()
    }
}

impl Default for ImageBuf {
    fn default() -> Self {
        ImageBuf::empty()
    }
}

#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
impl ImageBuf {
    /// Load an image from a DynamicImage from the image crate
    pub fn from_dynamic_image(image_data: image::DynamicImage) -> ImageBuf {
        fn has_alpha_channel(color: image::ColorType) -> bool {
            use image::ColorType::*;
            matches!(color, La8 | Rgba8 | La16 | Rgba16 | Bgra8)
        }

        let has_alpha_channel = has_alpha_channel(image_data.color());

        if has_alpha_channel {
            ImageBuf::from_dynamic_image_with_alpha(image_data)
        } else {
            ImageBuf::from_dynamic_image_without_alpha(image_data)
        }
    }

    /// Load an image from a DynamicImage with alpha
    pub fn from_dynamic_image_with_alpha(image_data: image::DynamicImage) -> ImageBuf {
        let rgba_image = image_data.to_rgba8();
        let sizeofimage = rgba_image.dimensions();
        ImageBuf::from_raw(
            rgba_image.to_vec(),
            ImageFormat::RgbaSeparate,
            sizeofimage.0 as usize,
            sizeofimage.1 as usize,
        )
    }

    /// Load an image from a DynamicImage without alpha
    pub fn from_dynamic_image_without_alpha(image_data: image::DynamicImage) -> ImageBuf {
        let rgb_image = image_data.to_rgb8();
        let sizeofimage = rgb_image.dimensions();
        ImageBuf::from_raw(
            rgb_image.to_vec(),
            ImageFormat::Rgb,
            sizeofimage.0 as usize,
            sizeofimage.1 as usize,
        )
    }

    /// Attempt to load an image from raw bytes.
    ///
    /// If the image crate can't decode an image from the data an error will be returned.
    pub fn from_data(raw_image: &[u8]) -> Result<ImageBuf, Box<dyn Error>> {
        let image_data = image::load_from_memory(raw_image).map_err(|e| e)?;
        Ok(ImageBuf::from_dynamic_image(image_data))
    }

    /// Attempt to load an image from the file at the provided path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<ImageBuf, Box<dyn Error>> {
        let image_data = image::open(path).map_err(|e| e)?;
        Ok(ImageBuf::from_dynamic_image(image_data))
    }
}

impl std::fmt::Debug for ImageBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ImageBuf")
            .field("size", &self.pixels.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &format_args!("{:?}", self.format))
            .finish()
    }
}
