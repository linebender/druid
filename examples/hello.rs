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

use druid::shell::{runloop, WindowBuilder};
use druid::widget::{ActionWrapper, Align, Button, Column, DynLabel, Padding};
use druid::{UiMain, UiState};

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut col = Column::new();
    let label = DynLabel::new(|data: &u32, _env| format!("value: {}", data));
    let button = Button::new("increment");
    col.add_child(Align::centered(Padding::uniform(5.0, label)), 1.0);
    col.add_child(Padding::uniform(5.0, button), 1.0);
    let root = ActionWrapper::new(col, |data: &mut u32, _env| *data += 1);
    let state = UiState::new(root, 0u32);
    builder.set_title("Hello example");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
