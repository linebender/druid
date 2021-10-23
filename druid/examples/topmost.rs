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

//! An example of the topmost window.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::prelude::*;
use druid::widget::{Flex, Label, Button};
use druid::{AppLauncher, WindowDesc, WidgetExt, Lens};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

#[derive(Clone, Default, Data, Lens)]
struct WindowState {
    is_topmost: bool,
}

pub fn main() {
    let window_state = WindowState {
        is_topmost: true,
    };

    let main_window = WindowDesc::new(build_root_widget())
        .title("Hello World!")
        .topmost(window_state.is_topmost)
        .window_size((400.0, 400.0));

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(window_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<WindowState> {
    let label = Label::new(|data: &bool, _: &Env| match data {
        false => "This window is not topmost.\nTry to choose other window".into(),
        true => "This window is topmost.\nTry to choose other window".into(),
    })
    .with_text_size(24.0)
    .lens(WindowState::is_topmost);

    let button = Button::new(|data: &bool, _: &Env| match data {
        false => "set topmost".into(),
        true => "set not topmost".into(),
    })
    .on_click(|ctx, data: &mut bool, _: &Env| {
        *data = !*data;
        ctx.window().topmost(*data);
    })
    .lens(WindowState::is_topmost);

    Flex::column()
        .with_child(label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(button)
}
