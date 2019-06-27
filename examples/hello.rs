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

use druid::{UiMain, UiState};
use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::{Button, Column, Padding};

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut root = Column::new();
    let button1 = Button::new("button1");
    let button2 = Button::new("button2");
    root.add_child(Padding::uniform(5.0, button1), (), 1.0);
    root.add_child(Padding::uniform(5.0, button2), (), 1.0);
    let state = UiState::new(root);
    builder.set_title("Hello example");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
