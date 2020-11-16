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

//! This example shows how to draw an png image.

use druid::{
    widget::{FillStrat, Flex, Image, WidgetExt},
    AppLauncher, Color, ImageBuf, Widget, WindowDesc,
};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder);
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<u32> {
    let png_data = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();

    let mut col = Flex::column();

    col.add_flex_child(
        Image::new(png_data.clone())
            .border(Color::WHITE, 1.0)
            .fix_width(100.0)
            .center(),
        1.0,
    );

    /*
    // If you want to change the fill stratagy you can but you need the widget to be mut
    let mut otherimage = Image::new(png_data);
    otherimage.set_fill(FillStrat::FitWidth);
    */

    let otherimage = Image::new(png_data)
        .fill_mode(FillStrat::FitWidth)
        .border(Color::WHITE, 1.0);
    col.add_flex_child(otherimage, 1.0);
    col
}
