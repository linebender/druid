// Copyright 2020 The xi-editor Authors.
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
//! Please consider using SVG and the SVG wideget as it scales much better.

use std::convert::AsRef;
use std::error::Error;
use std::marker::PhantomData;
use std::path::Path;

use image;

use crate::{
    piet::{ImageFormat, InterpolationMode},
    widget::common::FillStrat,
    Affine, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Rect, RenderContext, Size, UpdateCtx, Widget,
};

/// A widget that renders an Image
pub struct Image<T> {
    image_data: ImageData,
    phantom: PhantomData<T>,
    fill: FillStrat,
}

impl<T: Data> Image<T> {
    /// Create an image drawing widget from `ImageData`.
    ///
    /// The Image will scale to fit its box constraints.
    pub fn new(image_data: ImageData) -> Self {
        Image {
            image_data,
            phantom: Default::default(),
            fill: FillStrat::default(),
        }
    }

    /// A builder-style method for specifying the fill strategy.
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        self
    }

    /// Modify the widget's `FillStrat`.
    pub fn set_fill(&mut self, newfil: FillStrat) {
        self.fill = newfil;
    }
}

impl<T: Data> Widget<T> for Image<T> {
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

    fn paint(&mut self, paint_ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let offset_matrix = self
            .fill
            .affine_to_fill(paint_ctx.size(), self.image_data.get_size());

        // The ImageData's to_piet function does not clip to the image's size
        // CairoRenderContext is very like druids but with some extra goodies like clip
        if self.fill != FillStrat::Contain {
            let clip_rect = Rect::ZERO.with_size(paint_ctx.size());
            paint_ctx.clip(clip_rect);
        }
        self.image_data.to_piet(offset_matrix, paint_ctx);
    }
}

/// Stored Image data.
#[derive(Clone)]
pub struct ImageData {
    pixels: Vec<u8>,
    x_pixels: u32,
    y_pixels: u32,
}

impl ImageData {
    /// Create an empty Image
    pub fn empty() -> Self {
        ImageData {
            pixels: [].to_vec(),
            x_pixels: 0,
            y_pixels: 0,
        }
    }

    /// Load an image from a DynamicImage from the image crate
    pub fn from_dynamic_image(image_data: image::DynamicImage) -> ImageData {
        let rgb_image = image_data.to_rgb();
        let sizeofimage = rgb_image.dimensions();
        ImageData {
            pixels: rgb_image.to_vec(),
            x_pixels: sizeofimage.0,
            y_pixels: sizeofimage.1,
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

    /// Get the size in pixels of the contained image.
    fn get_size(&self) -> Size {
        Size::new(self.x_pixels as f64, self.y_pixels as f64)
    }

    /// Convert ImageData into Piet draw instructions
    fn to_piet(&self, offset_matrix: Affine, paint_ctx: &mut PaintCtx) {
        paint_ctx
            .with_save(|ctx| {
                ctx.transform(offset_matrix);

                let im = ctx
                    .make_image(
                        self.x_pixels as usize,
                        self.y_pixels as usize,
                        &self.pixels,
                        ImageFormat::Rgb,
                    )
                    .unwrap();
                let rec = Rect::from_origin_size(
                    (0.0, 0.0),
                    (self.x_pixels as f64, self.y_pixels as f64),
                );
                ctx.draw_image(&im, rec, InterpolationMode::Bilinear);

                Ok(())
            })
            .unwrap();
    }
}

impl Default for ImageData {
    fn default() -> Self {
        ImageData::empty()
    }
}
