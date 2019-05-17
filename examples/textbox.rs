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

//! Simple textbox example.

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::{Column, EventForwarder, KeyListener, Label, Padding, Row, Slider, TextBox};
use druid::{KeyEvent, KeyVariant, UiMain, UiState};

use druid::Id;

fn pad(widget: Id, state: &mut UiState) -> Id {
    Padding::uniform(5.0).ui(widget, state)
}

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();

    let mut column = Column::new();

    let text_box1 = pad(TextBox::new(None, 50.).ui(&mut state), &mut state);
    let text_box2 = pad(TextBox::new(None, 500.).ui(&mut state), &mut state);

    let slider_1 = pad(Slider::new(1.0).ui(&mut state), &mut state);
    let slider_2 = pad(Slider::new(0.5).ui(&mut state), &mut state);

    let panel = column.ui(&[text_box1, text_box2, slider_1, slider_2], &mut state);

    state.set_root(panel);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Text box");
    let window = builder.build().expect("built window");
    window.show();
    run_loop.run();
}
