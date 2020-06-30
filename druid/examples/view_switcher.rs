// Copyright 2020 The Druid Authors.
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

//! This example demonstrates the `ViewSwitcher` widget

use druid::widget::{Button, Flex, Label, Split, TextBox, ViewSwitcher};
use druid::{AppLauncher, Data, Env, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};

#[derive(Clone, Data, Lens)]
struct AppState {
    current_view: u32,
    current_text: String,
}

pub fn main() {
    let main_window = WindowDesc::new(make_ui).title(LocalizedString::new("View Switcher"));
    let data = AppState {
        current_view: 0,
        current_text: "Edit me!".to_string(),
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let mut switcher_column = Flex::column();
    switcher_column.add_child(
        Label::new(|data: &u32, _env: &Env| format!("Current view: {}", data))
            .lens(AppState::current_view),
    );
    for i in 0..6 {
        switcher_column.add_spacer(80.);
        switcher_column.add_child(
            Button::new(format!("View {}", i))
                .on_click(move |_event, data: &mut u32, _env| {
                    *data = i;
                })
                .lens(AppState::current_view),
        );
    }

    let view_switcher = ViewSwitcher::new(
        |data: &AppState, _env| data.current_view,
        |selector, _data, _env| match selector {
            0 => Box::new(Label::new("Simple Label").center()),
            1 => Box::new(
                Button::new("Simple Button").on_click(|_event, _data, _env| {
                    println!("Simple button clicked!");
                }),
            ),
            2 => Box::new(
                Button::new("Another Simple Button").on_click(|_event, _data, _env| {
                    println!("Another simple button clicked!");
                }),
            ),
            3 => Box::new(
                Flex::column()
                    .with_flex_child(Label::new("Here is a label").center(), 1.0)
                    .with_flex_child(
                        Button::new("Button").on_click(|_event, _data, _env| {
                            println!("Complex button clicked!");
                        }),
                        1.0,
                    )
                    .with_flex_child(TextBox::new().lens(AppState::current_text), 1.0)
                    .with_flex_child(
                        Label::new(|data: &String, _env: &Env| format!("Value entered: {}", data))
                            .lens(AppState::current_text),
                        1.0,
                    ),
            ),
            4 => Box::new(
                Split::columns(
                    Label::new("Left split").center(),
                    Label::new("Right split").center(),
                )
                .draggable(true),
            ),
            _ => Box::new(Label::new("Unknown").center()),
        },
    );

    Flex::row()
        .with_child(switcher_column)
        .with_flex_child(view_switcher, 1.0)
}
