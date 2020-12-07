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

//! This example shows how to draw an image. Using the Image feature.
//! Diferent image formats can be "unlocked" by adding their
//! corresponding feature.

use druid::widget::prelude::*;
use druid::widget::{FillStrat, Flex, Image, WidgetExt};
use druid::{AppLauncher, Color, ImageBuf, WindowDesc};
use druid::piet::InterpolationMode;

pub fn main() {
    let main_window = WindowDesc::new(ui_builder);
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(0)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<u32> {
    let png_data = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();

    // We create 2 images, one not having any modifications and the other
    // is set to a fixed width, a fill strategy and an interpolation mode.
    // You can see how this affects the final result. You can play with the
    // Interpolation mode to see hwo this affects things.
    // Note that this image is already anti-aliased so NearestNeighbor looks
    // weird
    Flex::column()
        .with_flex_child(
            Image::new(png_data.clone())
                .fill_mode(FillStrat::FitWidth)
                .interpolation_mode(InterpolationMode::NearestNeighbor)
                .border(Color::WHITE, 1.0)
                .fix_width(150.0)
                .center(),
            1.0,
        )
        .with_flex_child(
            Image::new(png_data)
                .fill_mode(FillStrat::FitWidth)
                .border(Color::WHITE, 1.0),
            1.0,
        )
}
