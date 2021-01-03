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

//! An example using the Either widget to show/hide a slider.
//! This is a very simple example, it uses a bool to determine
//! which widget gets shown.

use druid::widget::prelude::*;
use druid::widget::{Checkbox, Either, Flex, Label, Slider};
use druid::{AppLauncher, Data, Lens, WidgetExt, WindowDesc};

#[derive(Clone, Default, Data, Lens)]
struct AppState {
    which: bool,
    value: f64,
}

fn ui_builder() -> impl Widget<AppState> {
    // Our UI consists of a column with a button and an `Either` widget
    let button = Checkbox::new("Toggle slider")
        .lens(AppState::which)
        .padding(5.0);

    // The `Either` widget has two children, only one of which is visible at a time.
    // To determine which child is visible, you pass it a closure that takes the
    // `Data` and the `Env` and returns a bool; if it returns `true`, the first
    // widget will be visible, and if `false`, the second.
    let either = Either::new(
        |data, _env| data.which,
        Slider::new().lens(AppState::value).padding(5.0),
        Label::new("Click to reveal slider").padding(5.0),
    );
    Flex::column().with_child(button).with_child(either)
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title("Switcheroo");
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(AppState::default())
        .expect("launch failed");
}
