// Copyright 2020 The xi-editor Authors.
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

use druid::widget::{Button, Controller, Flex, Label, TextBox};
use druid::{
    AppLauncher, Color, Data, DialogDesc, Env, Event, EventCtx, KeyCode, Lens, ModalDesc, Widget,
    WidgetExt, WindowDesc,
};

const WINDOW_TITLE: &str = "Number entry";

#[derive(Clone, Data, Lens)]
struct ModalState {
    number: String,
}

struct NumberEntryController;

fn make_modal() -> ModalDesc<ModalState> {
    let label = Label::new("Only numbers allowed!");
    let button = Button::new("I'm sorry, it won't happen again.")
        .on_click(|ctx, _data, _env| {
            ctx.dismiss_modal();
        })
        .tooltip("Go on, apologize.");
    let flex = Flex::column()
        .with_child(label)
        .with_child(button)
        .center()
        .expand()
        .background(Color::grey8(200).with_alpha(0.8));
    ModalDesc::new(flex)
}

impl<W: Widget<String>> Controller<String, W> for NumberEntryController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
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
                _ => ctx.show_modal(make_modal()),
            }
        } else {
            child.event(ctx, event, data, env);
        }
    }
}

pub fn main() {
    let main_window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    let initial_state = ModalState {
        number: "123".into(),
    };

    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<ModalState> {
    let textbox = TextBox::new()
        .controller(NumberEntryController)
        .lens(ModalState::number);

    let button = Button::new("Clear").on_click(|ctx, _data, _env| {
        ctx.show_dialog(
            DialogDesc::new("Really clear?")
                .background(Color::grey8(100).with_alpha(0.8))
                .with_option("Yes, really", |_ctx, data: &mut ModalState, _env| {
                    data.number.clear();
                })
                .with_option("Never mind", |_, _, _| {}),
        );
    });

    Flex::column()
        .with_child(textbox)
        .with_child(button)
        .center()
}
