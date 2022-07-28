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

//! This is a very small example of how to setup a druid application.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::prelude::*;
use druid::widget::{Button, Label, LinearVec2, ZStack};
use druid::{AppLauncher, Data, Lens, WidgetExt, WindowDesc};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 200.0;

#[derive(Clone, Data, Lens)]
struct State {
    counter: usize,
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Hello World!")
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state: State = State {
        counter: 0,
    };

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<State> {
    ZStack::new(
        Button::from_label(Label::dynamic(|state: &State, _|format!("Very large button with text! Count up (currently {})", state.counter)))
            .on_click(|_, state: &mut State, _|state.counter += 1)
    ).with_child_at_index(
        Button::new("Reset").on_click(|_, state: &mut State, _|state.counter = 0),
        LinearVec2::new((0.0, 0.5), (10.0, 0.0)),
        LinearVec2::full(),

        0,

    )
}
