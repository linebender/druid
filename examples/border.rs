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

//! Example to play around with container and border.

use druid::shell::piet::Color;
use druid::widget::{Button, Column, Container, Label, Padding, Row, SizedBox};
use druid::{AppLauncher, Widget, WindowDesc};

fn build_app() -> impl Widget<u32> {
    let mut row = Row::new();
    row.add_child(
        Container::new()
            .color(Color::rgb8(0, 0x77, 0x77))
            .child(SizedBox::empty().expand()),
        1.0,
    );

    let mut col = Column::new();
    col.add_child(
        Container::new()
            .color(Color::rgb8(0x77, 0, 0x77))
            .child(SizedBox::empty().expand()),
        1.0,
    );
    col.add_child(
        Container::new().border(Color::WHITE, 20.0).child(
            SizedBox::new(
                Container::new()
                    .color(Color::rgb8(0x77, 0x77, 0))
                    .child(Label::new("Hello world")),
            )
            .expand(),
        ),
        1.0,
    );

    row.add_child(col, 1.0);

    let root = Container::new()
        .color(Color::BLACK)
        .padding(30.0)
        .child(row);

    root
}

fn main() {
    let window = WindowDesc::new(build_app);
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0u32)
        .expect("launch failed");
}
