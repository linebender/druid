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
use druid::widget::{Flex, Label};
use druid::{AppLauncher, WindowDesc};

pub fn main() {
    let main_window = WindowDesc::new(build_root_widget())
        .title("Hello World!")
        // Set this window to the topmost.
        .topmost(true)
        .window_size((400.0, 400.0));

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(())
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<()> {
    let label = Label::new("This window is topmost.")
        .with_text_size(24.0);

    Flex::column()
        .with_child(label)
}
