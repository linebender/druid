// Copyright 2019 The Druid Authors.
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

//! This example shows how to construct a basic layout,
//! using columns, rows, and loops, for repeated Widgets.

use druid::widget::{Button, Flex, Label};
use druid::{AppLauncher, Color, Widget, WidgetExt, WindowDesc};

fn build_app() -> impl Widget<u32> {
    // Usually we put all the widgets in one big tree using builder-style
    // methods. Sometimes we split them up in declarations to increase
    // readability. In this case we also have some recurring elements,
    // we add those in a loop later on.
    let mut col = Flex::column().with_child(
        // The `Flex`'s first child is another Flex! In this case it is
        // a row.
        Flex::row()
            // The row has its own children.
            .with_child(
                Label::new("One")
                    .fix_width(60.0)
                    .background(Color::rgb8(0x77, 0x77, 0))
                    .border(Color::WHITE, 3.0)
                    .center(),
            )
            // Spacing element that will fill all available space in
            // between label and a button. Notice that weight is non-zero.
            // We could have achieved a similair result with expanding the
            // width and setting the main-axis-allignment to SpaceBetween.
            .with_flex_spacer(1.0)
            .with_child(Button::new("Two").padding(20.))
            // After we added all the children, we can set some more
            // values using builder-style methods. Since these methods
            // dont return the original `Flex` but a SizedBox and Container
            // respectively, we have to put these at the end.
            .fix_height(100.0)
            //turquoise
            .background(Color::rgb8(0, 0x77, 0x88)),
    );

    for i in 0..5 {
        // Give a larger weight to one of the buttons for it to
        // occupy more space.
        let weight = if i == 2 { 3.0 } else { 1.0 };
        // call `expand_height` to force the buttons to use all their provided flex
        col.add_flex_child(
            Button::new(format!("Button #{}", i)).expand_height(),
            weight,
        );
    }
    // This method asks druid to draw colored rectangles around our widgets,
    // so we can visually inspect their layout rectangles.
    col.debug_paint_layout()
}

pub fn main() {
    let window = WindowDesc::new(build_app()).title("Very flexible");

    AppLauncher::with_window(window)
        .log_to_console()
        .launch(0)
        .expect("launch failed");
}
