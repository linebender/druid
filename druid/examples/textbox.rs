// Copyright 2021 The Druid Authors.
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

//! An example of various text layout features.
//!
//! I would like to make this a bit fancier (like the flex demo) but for now
//! lets keep it simple.

use std::sync::Arc;

use druid::widget::{Flex, Label, TextBox};
use druid::{
    AppLauncher, Color, Data, Env, Lens, LocalizedString, Menu, Widget, WidgetExt, WindowDesc,
    WindowId,
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Text Options");

const EXPLAINER: &str = "\
    This example demonstrates some of the possible configurations \
    of the TextBox widget.\n\
    The top textbox allows a single line of input, with horizontal scrolling \
    but no scrollbars. The bottom textbox allows mutliple lines of text, wrapping \
    words to fit the width, and allowing vertical scrolling when it runs out \
    of room to grow vertically.";

#[derive(Clone, Data, Lens)]
struct AppState {
    multi: Arc<String>,
    single: Arc<String>,
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .menu(make_menu)
        .window_size((400.0, 600.0));

    // create the initial app state
    let initial_state = AppState {
        single: "".to_string().into(),
        multi: "".to_string().into(),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    let blurb = Label::new(EXPLAINER)
        .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
        .padding(8.0)
        .border(Color::grey(0.6), 2.0)
        .rounded(5.0);

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(blurb)
        .with_spacer(24.0)
        .with_child(
            TextBox::new()
                .with_placeholder("Single")
                .lens(AppState::single),
        )
        .with_default_spacer()
        .with_flex_child(
            TextBox::multiline()
                .with_placeholder("Multi")
                .lens(AppState::multi)
                .expand_width(),
            1.0,
        )
        .padding(8.0)
}

#[allow(unused_assignments, unused_mut)]
fn make_menu<T: Data>(_window: Option<WindowId>, _data: &AppState, _env: &Env) -> Menu<T> {
    let mut base = Menu::empty();
    #[cfg(target_os = "macos")]
    {
        base = base.entry(druid::platform_menus::mac::application::default())
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        base = base.entry(druid::platform_menus::win::file::default());
    }
    base.entry(
        Menu::new(LocalizedString::new("common-menu-edit-menu"))
            .entry(druid::platform_menus::common::undo())
            .entry(druid::platform_menus::common::redo())
            .separator()
            .entry(druid::platform_menus::common::cut())
            .entry(druid::platform_menus::common::copy())
            .entry(druid::platform_menus::common::paste()),
    )
}
