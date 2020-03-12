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

//! Demos the textbox widget, as well as menu creation and overriding theme settings.

use druid::widget::{EnvScope, Flex, Label, Padding, TextBox};
use druid::{theme, AppLauncher, Color, Data, LocalizedString, MenuDesc, Widget, WindowDesc};

fn main() {
    let window = WindowDesc::new(build_widget).menu(make_main_menu()).title(
        LocalizedString::new("textbox-demo-window-title").with_placeholder("typing is fun!"),
    );
    AppLauncher::with_window(window)
        .configure_env(|env, _| {
            env.set(theme::SELECTION_COLOR, Color::rgb8(0xA6, 0xCC, 0xFF));
            env.set(theme::WINDOW_BACKGROUND_COLOR, Color::WHITE);
            env.set(theme::LABEL_COLOR, Color::BLACK);
            env.set(theme::CURSOR_COLOR, Color::BLACK);
            env.set(theme::BACKGROUND_LIGHT, Color::rgb8(230, 230, 230));
        })
        .use_simple_logger()
        .launch("typing is fun!".to_string())
        .expect("launch failed");
}

fn build_widget() -> impl Widget<String> {
    let textbox = TextBox::new();
    let textbox_2 = EnvScope::new(
        |env, _| {
            env.set(theme::BACKGROUND_LIGHT, Color::rgb8(50, 50, 50));
            env.set(theme::LABEL_COLOR, Color::WHITE);
            env.set(theme::CURSOR_COLOR, Color::WHITE);
            env.set(theme::SELECTION_COLOR, Color::rgb8(100, 100, 100));
        },
        TextBox::new().with_placeholder("placeholder"),
    );
    let label = Label::new(|data: &String, _env: &_| format!("value: {}", data));

    Flex::column()
        .with_child(Padding::new(5.0, textbox), 1.0)
        .with_child(Padding::new(5.0, textbox_2), 1.0)
        .with_child(Padding::new(5.0, label), 1.0)
}

fn make_main_menu<T: Data>() -> MenuDesc<T> {
    let edit_menu = MenuDesc::new(LocalizedString::new("common-menu-edit-menu"))
        .append(druid::platform_menus::common::cut())
        .append(druid::platform_menus::common::copy())
        .append(druid::platform_menus::common::paste());

    MenuDesc::platform_default()
        .unwrap_or(MenuDesc::empty())
        .append(edit_menu)
}
