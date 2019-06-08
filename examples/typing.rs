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

//! Simple typing test.

extern crate druid;
extern crate druid_shell;

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::{Column, EventForwarder, KeyListener, Label, Padding};
use druid::{KeyCode, KeyEvent, UiMain, UiState};

use druid::Id;

struct TypingState {
    value: String,
    cursor: usize,
}

#[derive(Debug, Clone)]
enum TypingAction {
    Append(String),
    Delete(),
}

impl TypingState {
    fn action(&mut self, action: &TypingAction) {
        match action {
            TypingAction::Append(ch) => self.append(&ch),
            TypingAction::Delete() => self.delete(),
        }
    }

    fn move_cursor(&mut self) {}

    fn append(&mut self, s: &str) {
        self.value.push_str(s);
    }

    fn delete(&mut self) {
        self.value.pop();
    }
}

fn pad(widget: Id, ui: &mut UiState) -> Id {
    Padding::uniform(5.0).ui(widget, ui)
}

fn build_typing(ui: &mut UiState) {
    let display = Label::new("".to_string()).ui(ui);
    let row0 = pad(display, ui);

    let column = Column::new();

    let panel = column.ui(&[row0], ui);
    let key_listener = KeyListener::new().ui(panel, ui);
    let forwarder = EventForwarder::<TypingAction>::new().ui(key_listener, ui);
    let mut typing_state = TypingState {
        value: "".to_string(),
        cursor: 0,
    };

    ui.add_listener(key_listener, move |event: &mut KeyEvent, mut ctx| {
        if let Some(mut action) = action_for_key(event) {
            ctx.poke_up(&mut action);
        }
    });

    ui.add_listener(forwarder, move |action: &mut TypingAction, mut ctx| {
        typing_state.action(action);
        ctx.poke(display, &mut typing_state.value);
    });

    let root = pad(forwarder, ui);
    ui.set_root(root);
    ui.set_focus(Some(key_listener));
}

fn action_for_key(event: &KeyEvent) -> Option<TypingAction> {
    match event {
        KeyEvent::Character(data) => Some(TypingAction::Append(
            data.text().map(String::from).unwrap_or_default(),
        )),
        KeyEvent::NonCharacter(data) if data.key_code == KeyCode::Backspace => {
            Some(TypingAction::Delete())
        }
        _other => None,
    }
}

fn main() {
    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    build_typing(&mut state);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Typing!");
    let window = builder.build().expect("built window");
    window.show();
    run_loop.run();
}
