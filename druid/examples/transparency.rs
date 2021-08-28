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

//! An example of a transparent window background.
//! Useful for dropdowns, tooltips and other overlay windows.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::kurbo::Circle;
use druid::widget::prelude::*;
use druid::widget::{Flex, Label, Painter, TextBox, WidgetExt};
use druid::{AppLauncher, Color, Lens, Rect, WindowDesc};

#[derive(Clone, Data, Lens)]
struct HelloState {
    name: String,
}

pub fn main() {
    let window = WindowDesc::new(build_root_widget())
        .show_titlebar(false)
        .window_size((512., 512.))
        .transparent(true)
        .resizable(true)
        .title("Transparent background");

    AppLauncher::with_window(window)
        .log_to_console()
        .launch(HelloState { name: "".into() })
        .expect("launch failed");
}

fn build_root_widget() -> impl Widget<HelloState> {
    // Draw red circle, and two semi-transparent rectangles
    let circle_and_rects = Painter::new(|ctx, _data, _env| {
        let boundaries = ctx.size().to_rect();
        let center = (boundaries.width() / 2., boundaries.height() / 2.);
        let circle = Circle::new(center, center.0.min(center.1));
        ctx.fill(circle, &Color::RED);

        let rect1 = Rect::new(0., 0., boundaries.width() / 2., boundaries.height() / 2.);
        ctx.fill(rect1, &Color::rgba8(0x0, 0xff, 0, 125));

        let rect2 = Rect::new(
            boundaries.width() / 2.,
            boundaries.height() / 2.,
            boundaries.width(),
            boundaries.height(),
        );
        ctx.fill(rect2, &Color::rgba8(0x0, 0x0, 0xff, 125));
    });

    // This textbox modifies the label, idea here is to test that the background
    // invalidation works when you type to the textbox
    let textbox = TextBox::new()
        .with_placeholder("Type to test clearing")
        .with_text_size(18.0)
        .lens(HelloState::name)
        .fix_width(250.);

    let label = Label::new(|data: &HelloState, _env: &Env| {
        if data.name.is_empty() {
            "Text: ".to_string()
        } else {
            format!("Text: {}!", data.name)
        }
    })
    .with_text_color(Color::RED)
    .with_text_size(32.0);

    Flex::column()
        .with_flex_child(circle_and_rects.expand(), 10.0)
        .with_spacer(4.0)
        .with_child(textbox)
        .with_spacer(4.0)
        .with_child(label)
}
