// Copyright 2018 The xi-editor Authors.
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

//! Example of sending external events to the UI.

extern crate druid;
extern crate druid_win_shell;

use std::{thread, time};

use druid_win_shell::win_main;
use druid_win_shell::window::WindowBuilder;

use druid::widget::Label;
use druid::{UiMain, UiState};

fn main() {
    druid_win_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let label = Label::new("Initial state").ui(&mut state);
    state.set_root(label);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ext event example");
    let window = builder.build().unwrap();
    let idle_handle = window.get_idle_handle().unwrap();

    // This will be set from the idle handler, updated just after the window is shown.
    UiMain::send_ext(&idle_handle, label, "New state".to_string());

    // Illustration of injecting events from another thread.
    thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        UiMain::send_ext(&idle_handle, label, "State updated from thread".to_string());
    });

    window.show();
    run_loop.run();
}
