// Copyright 2019 The Druid Authors.
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

//! This demonstrate how to create a custom title bar. Please note that this currently only
//! works on Windows and Gtk.

use druid::widget::{Flex, Label, WindowDragHandle};
use druid::{AppLauncher, Color, Widget, WidgetExt, WindowDesc};

pub fn main() {
    //Create a new window without the default title bar of the platform.
    let window = WindowDesc::new(build_root_widget()).show_titlebar(false);

    //Launch the application.
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(())
        .expect("launch failed");
}

fn build_root_widget() -> impl Widget<()> {
    Flex::column().with_child(build_title_bar())
}

fn build_title_bar() -> impl Widget<()> {
    use druid::commands::{CLOSE_WINDOW, CONFIGURE_WINDOW};
    use druid::{WindowConfig, WindowState};

    //Let's create a custom title bar. We need a label for the title, a minimize button, a maximize
    //button and an exit button.
    let title_label = Label::new("My Application").padding((5.0, 5.0));
    let minimize_button = Label::new("🗕")
        .padding((5.0, 5.0))
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(
                CONFIGURE_WINDOW
                    .with(WindowConfig::default().set_window_state(WindowState::Minimized)),
            )
        });
    let maximize_button = Label::new("🗖")
        .padding((5.0, 5.0))
        .on_click(|ctx, _data, _env| {
            ctx.submit_command(
                CONFIGURE_WINDOW
                    .with(WindowConfig::default().set_window_state(WindowState::Maximized)),
            )
        });
    let exit_button = Label::new("✕")
        .padding((5.0, 5.0))
        .on_click(|ctx, _data, _env| ctx.submit_command(CLOSE_WINDOW));

    //Wrap the title label with the WindowDragHandle controller widget. Do not wrap the buttons,
    //because not only dragging from the buttons is strange, but on most platforms, clicks events
    //aren't passed though the drag handle.
    let drag_handle = WindowDragHandle::new(title_label);

    //Here, we use a Flex widget for the layout.
    Flex::row()
        .with_flex_child(drag_handle.expand_width(), 1.0)
        .with_child(minimize_button)
        .with_child(maximize_button)
        .with_child(exit_button)
        .background(Color::from_rgba32_u32(0x2660A4_FF))
}
