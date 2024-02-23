// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This example demonstrates the `Split` widget

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::piet::Color;
use druid::widget::{Align, Container, Label, Padding, Split};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

fn build_app() -> impl Widget<u32> {
    let fixed_cols = Padding::new(
        10.0,
        Container::new(
            Split::columns(
                Align::centered(Label::new("Left Split")),
                Align::centered(Label::new("Right Split")),
            )
            .split_point(0.5),
        )
        .border(Color::WHITE, 1.0),
    );
    let fixed_rows = Padding::new(
        10.0,
        Container::new(
            Split::rows(
                Align::centered(Label::new("Top Split")),
                Align::centered(Label::new("Bottom Split")),
            )
            .split_point(0.4)
            .bar_size(3.0),
        )
        .border(Color::WHITE, 1.0),
    );
    let draggable_cols = Padding::new(
        10.0,
        Container::new(
            Split::columns(
                Align::centered(Label::new("Split A")),
                Align::centered(Label::new("Split B")),
            )
            .split_point(0.5)
            .draggable(true)
            .solid_bar(true)
            .min_size(60.0, 60.0),
        )
        .border(Color::WHITE, 1.0),
    );
    Padding::new(
        10.0,
        Container::new(
            Split::rows(
                Split::rows(fixed_cols, fixed_rows)
                    .split_point(0.33)
                    .bar_size(3.0)
                    .min_bar_area(3.0)
                    .draggable(true),
                draggable_cols,
            )
            .split_point(0.75)
            .bar_size(5.0)
            .min_bar_area(11.0)
            .draggable(true),
        )
        .border(Color::WHITE, 1.0),
    )
}

pub fn main() {
    let window = WindowDesc::new(build_app())
        .title(LocalizedString::new("split-demo-window-title").with_placeholder("Split Demo"));
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(0u32)
        .expect("launch failed");
}
