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
use druid::widget::{Button, Column, Padding, Scroll};
use druid::{UiMain, UiState};

fn main() {
    druid::shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut col = Column::new();
    for i in 0..30 {
        let button = Button::new(format!("Button {}", i));
        col.add_child(Padding::uniform(3.0, button), 0.0);
    }
    let scroll = Scroll::new(col);
    let state = UiState::new(scroll, 0u32);
    builder.set_title("Scroll example");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
