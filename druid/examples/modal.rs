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

use druid::widget::{Align, Button, Controller, Flex, Label, TextBox};
use druid::{
    AppLauncher, Color, Data, Env, Event, EventCtx, KeyCode, Lens, Modal, SingleUse, Widget,
    WidgetExt, WindowDesc,
};

const WINDOW_TITLE: &'static str = "Number entry";

#[derive(Clone, Data, Lens)]
struct ModalState {
    number: String,
}

struct NumberEntryController;

fn make_modal() -> Modal<ModalState> {
    let label = Label::new("Only numbers allowed!");
    let button = Button::new("I'm sorry, it won't happen again.").on_click(|ctx, _data, _env| {
        ctx.submit_command(Modal::DISMISS_MODAL, None);
    });
    let flex = Flex::column().with_child(label).with_child(button);
    Modal::new(flex).background(Color::grey8(100).with_alpha(0.5))
}

impl<T, W: Widget<T>> Controller<T, W> for NumberEntryController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::KeyDown(ev) = event {
            match ev.key_code {
                KeyCode::Key0
                | KeyCode::Key1
                | KeyCode::Key2
                | KeyCode::Key3
                | KeyCode::Key4
                | KeyCode::Key5
                | KeyCode::Key6
                | KeyCode::Key7
                | KeyCode::Key8
                | KeyCode::Key9
                | KeyCode::Backspace => child.event(ctx, event, data, env),
                _ => ctx.submit_command(Modal::SHOW_MODAL.with(SingleUse::new(make_modal())), None),
            }
        } else {
            child.event(ctx, event, data, env);
        }
    }
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = ModalState {
        number: "123".into(),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<ModalState> {
    let textbox = TextBox::new()
        .lens(ModalState::number)
        .controller(NumberEntryController);

    Align::centered(textbox)
}
