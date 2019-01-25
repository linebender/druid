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

//! An example of dynamic graph mutation.

extern crate direct2d;
extern crate directwrite;
extern crate druid;
extern crate druid_win_shell;

use std::collections::BTreeMap;

use druid_win_shell::win_main;
use druid_win_shell::window::WindowBuilder;

use druid::widget::{Button, Column, EventForwarder, Label, Padding, Row};
use druid::{Id, UiMain, UiState};

#[derive(Default)]
struct AppState {
    count: usize,
    buttons: BTreeMap<usize, Id>,
    selected: Option<usize>,
}

#[derive(Clone)]
enum Action {
    AddButton,
    DelButton,
    Sort,
    Select(usize),
}

fn main() {
    druid_win_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    let label = Label::new("Selection: None").ui(&mut state);
    let row1 = Row::new().ui(&[], &mut state);
    let add_button = Button::new("Add").ui(&mut state);
    let button1 = Padding::uniform(10.0).ui(add_button, &mut state);
    let del_button = Button::new("Del").ui(&mut state);
    let button2 = Padding::uniform(10.0).ui(del_button, &mut state);
    let sort_button = Button::new("Sort").ui(&mut state);
    let button3 = Padding::uniform(10.0).ui(sort_button, &mut state);
    let row2 = Row::new().ui(&[button1, button2, button3], &mut state);
    let col = Column::new().ui(&[label, row1, row2], &mut state);
    let forwarder = EventForwarder::<Action>::new().ui(col, &mut state);
    state.set_root(forwarder);
    let mut app = AppState::default();
    state.add_listener(
        forwarder,
        move |action: &mut Action, mut ctx| match action {
            Action::AddButton => {
                let n = app.count;
                app.count += 1;
                let label = format!("{}", n);
                let new_button = Button::new(label).ui(&mut ctx);
                println!("button {} id={}", n, new_button);
                ctx.add_listener(new_button, move |_: &mut bool, mut ctx| {
                    ctx.poke_up(&mut Action::Select(n));
                });
                let padded = Padding::uniform(10.0).ui(new_button, &mut ctx);
                app.buttons.insert(n, padded);
                if let Some(sibling) = app.selected {
                    ctx.add_before(row1, app.buttons[&sibling], padded);
                } else {
                    ctx.append_child(row1, padded);
                }
            }
            Action::DelButton => {
                if let Some(n) = app.selected.take() {
                    let id = app.buttons.remove(&n).unwrap();
                    ctx.delete_child(row1, id);
                    ctx.poke(label, &mut format!("Selection: {:?}", app.selected));
                }
            }
            Action::Sort => {
                for &child in app.buttons.values() {
                    ctx.remove_child(row1, child);
                    ctx.append_child(row1, child);
                }
            }
            Action::Select(n) => {
                app.selected = Some(*n);
                ctx.poke(label, &mut format!("Selection: {:?}", app.selected));
            }
        },
    );
    state.add_listener(add_button, move |_: &mut bool, mut ctx| {
        ctx.poke_up(&mut Action::AddButton);
    });
    state.add_listener(del_button, move |_: &mut bool, mut ctx| {
        ctx.poke_up(&mut Action::DelButton);
    });
    state.add_listener(sort_button, move |_: &mut bool, mut ctx| {
        ctx.poke_up(&mut Action::Sort);
    });
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Dynamic example");
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
