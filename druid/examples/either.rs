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

pub fn main() {
    let main_window =
        WindowDesc::new(ui_builder).title(druid::widget::LabelText::from("Switcheroo"));
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(AppState::default())
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppState> {
    // Our UI consists of a column with a button and an `Either` widget
    let button = Checkbox::new("Toggle slider")
        .lens(AppState::which)
        .padding(5.0);

    // The either widget has 2 children, one of which is visible.
    // you have to pass in a closure which takes `Data` and and `Env`.
    // This closure determines which widget gets displayed based on the
    // return value (a bool). False is the second widget and true is the first.
    let either = Either::new(
        |data, _env| data.which,
        Slider::new().lens(AppState::value).padding(5.0),
        Label::new("Click to reveal slider").padding(5.0),
    );
    Flex::column().with_child(button).with_child(either)
}
