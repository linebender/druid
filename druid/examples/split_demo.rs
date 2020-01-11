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

//! This example demonstrates the `Split` widget

use druid::piet::Color;
use druid::widget::{Align, Container, Label, Padding, Split};
use druid::{AppLauncher, Widget, WindowDesc};

fn build_app() -> impl Widget<u32> {
    let fixed_horizontal = Padding::new(
        10.0,
        Container::new(
            Split::horizontal(
                Align::centered(Label::new("Left Split")),
                Align::centered(Label::new("Right Split")),
            )
            .split_point(0.5),
        )
        .border(Color::WHITE, 1.0),
    );
    let fixed_vertical = Padding::new(
        10.0,
        Container::new(
            Split::vertical(
                Align::centered(Label::new("Top Split")),
                Align::centered(Label::new("Bottom Split")),
            )
            .split_point(0.4)
            .splitter_size(7.0),
        )
        .border(Color::WHITE, 1.0),
    );
    let draggable_horizontal = Padding::new(
        10.0,
        Container::new(
            Split::horizontal(
                Align::centered(Label::new("Split A")),
                Split::horizontal(
                    Align::centered(Label::new("Split B")),
                    Align::centered(Label::new("Split C")),
                )
                .draggable(true),
            )
            .split_point(0.33)
            .draggable(true),
        )
        .border(Color::WHITE, 1.0),
    );
    let draggable_vertical = Padding::new(
        10.0,
        Container::new(
            Split::vertical(
                Split::vertical(fixed_horizontal, fixed_vertical)
                    .split_point(0.33)
                    .splitter_size(5.0)
                    .draggable(true),
                draggable_horizontal,
            )
            .split_point(0.75)
            .splitter_size(5.0)
            .draggable(true),
        )
        .border(Color::WHITE, 1.0),
    );
    draggable_vertical
}

fn main() {
    let window = WindowDesc::new(build_app);
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(0u32)
        .expect("launch failed");
}
