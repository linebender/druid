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

use druid::widget::{Column, EventForwarder, KeyListener, Label, Padding, Row, Slider, TextBox, ProgressBar};
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

    let column = Column::new();

    let text_box1 = pad(TextBox::new(None, 50.).ui(&mut state), &mut state);
    let text_box2 = pad(TextBox::new(None, 500.).ui(&mut state), &mut state);

    let slider_1 = Slider::new(1.0).ui(&mut state);
    let slider_1_padded = pad(slider_1, &mut state);

    let slider_2 = Slider::new(0.5).ui(&mut state);
    let slider_2_padded = pad(slider_2, &mut state);

    let label_1 = Label::new("1.00").ui(&mut state);
    let label_1_padded = pad(label_1, &mut state);

    let label_2 = Label::new("0.50").ui(&mut state);
    let label_2_padded = pad(label_2, &mut state);

    let progress_bar_1 = ProgressBar::new(0.0).ui(&mut state);
    let progress_bar_1_padded = pad(progress_bar_1, &mut state);

    let mut row_1 = Row::new();
    let mut row_2 = Row::new();
    let mut row_3 = Row::new();

    row_1.set_flex(slider_1_padded, 1.0);
    row_2.set_flex(slider_2_padded, 1.0);
    row_3.set_flex(progress_bar_1_padded, 1.0);

    let row_1 = row_1.ui(&[slider_1_padded, label_1_padded], &mut state);
    let row_2 = row_2.ui(&[slider_2_padded, label_2_padded], &mut state);
    let row_3 = row_3.ui(&[progress_bar_1_padded], &mut state);

    let panel = column.ui(&[text_box1, text_box2, row_1, row_2, row_3], &mut state);

    state.add_listener(slider_1, move |value: &mut f64, mut ctx| {
        ctx.poke(progress_bar_1, value);
        ctx.poke(label_1, &mut format!("{:.2}", value));
    });

    state.add_listener(slider_2, move |value: &mut f64, mut ctx| {
        ctx.poke(label_2, &mut format!("{:.2}", value));
    });

    state.set_root(panel);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Text box");
    let window = builder.build().expect("built window");
    window.show();
    run_loop.run();
}
