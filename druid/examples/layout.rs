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

//! This example shows how to construct a basic layout.

use druid::widget::{Button, Flex, Label, SizedBox, WidgetExt};
use druid::{AppLauncher, Color, Widget, WindowDesc};

fn build_app() -> impl Widget<u32> {
    // Begin construction of vertical layout
    let mut col = Flex::column();

    // Construct a horizontal layout.
    let mut header = Flex::row();
    header.add_child(
        Label::new("One")
            .center()
            .fix_width(60.0)
            .background(Color::rgb8(0x77, 0x77, 0))
            .border(Color::WHITE, 3.0),
        0.0,
    );
    // Spacing element that will fill all available space in between label
    // and a button. Notice that weight is non-zero.
    header.add_child(SizedBox::empty().expand(), 1.0);
    header.add_child(Button::new("Two", Button::noop).padding(20.), 0.0);
    col.add_child(
        header
            .fix_height(100.0)
            .background(Color::rgb8(0, 0x77, 0x88)),
        0.0,
    );

    for i in 0..5 {
        // Give a larger weight to one of the buttons for it to
        // occupy more space.
        let weight = if i == 2 { 3.0 } else { 1.0 };
        col.add_child(Button::new(format!("Button #{}", i), Button::noop), weight);
    }

    col
}

fn main() {
    let window = WindowDesc::new(build_app);
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0u32)
        .expect("launch failed");
}
