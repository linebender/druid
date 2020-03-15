// Copyright 2019 The xi-editor Authors.
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

//! This example shows how to construct a stack layout.

use druid::widget::{Button, Image, ImageData, Stack, WidgetExt};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

fn build_app() -> impl Widget<u8> {
    // Begin construction of vertical layout
    let simple_data = ImageData::from_file("examples/PicWithAlpha.png").unwrap();
    let dog_data = ImageData::from_file("examples/dog.jpg").unwrap();
    let dog_img = Image::new(dog_data);
    let simple_img = Image::new(simple_data);
    Stack::new()
        .with_child(dog_img)
        .with_child(simple_img)
        .with_child(
            Button::new("Button 1", Button::noop)
                .fix_height(300.)
                .fix_width(300.),
        )
        .with_child(Button::new("Button 2", Button::noop))
}

fn main() {
    let window = WindowDesc::new(build_app)
        .window_size((800., 800.))
        .title(LocalizedString::new("layout-demo-window-title").with_placeholder("Stacked"));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0u8)
        .expect("launch failed");
}
