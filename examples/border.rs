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
use druid::shell::{runloop, WindowBuilder};
use druid::widget::{Button, Column, Container, Label, Padding, Row, SizedBox};
use druid::{UiMain, UiState, Widget};

fn build_app() -> impl Widget<u32> {
    let mut row = Row::new();
    row.add_child(
        Container::new(SizedBox::empty().expand()).color(Color::rgb8(0, 0x77, 0x77)),
        1.0,
    );

    let mut col = Column::new();
    col.add_child(
        Container::new(SizedBox::empty().expand()).color(Color::rgb8(0x77, 0, 0x77)),
        1.0,
    );
    col.add_child(
        Container::new(
            SizedBox::new(
                Container::new(Label::new("Hello world")).color(Color::rgb8(0x77, 0x77, 0)),
            )
            .expand(),
        )
        .border(Color::WHITE, 20.0),
        1.0,
    );

    row.add_child(col, 1.0);

    let root = Container::new(Padding::uniform(30.0, row)).color(Color::BLACK);

    root
}

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();

    // Build app layout
    let root = build_app();
    // Set up initial app state
    let state = UiState::new(root, 0u32);
    builder.set_title("Border example");
    builder.set_handler(Box::new(UiMain::new(state)));

    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
