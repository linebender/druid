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

//! This is a very small example of how to use menus.
//! It does the almost bare minimum while still being useful.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::prelude::*;
use druid::widget::{Flex, Label};
use druid::{AppDelegate, AppLauncher, Command, Data, DelegateCtx, Handled, Lens, Menu, MenuItem, Selector, Target, WidgetExt, WindowDesc};

const COMMAND: Selector = Selector::new("custom_Selector");

#[derive(Clone, Data, Lens)]
struct AppState {
    option: bool,
    value: usize,
}

pub fn main() {


    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Hello World!")
        .window_size((400.0, 400.0))
        .menu(|_, _, _|build_menu());

    // create the initial app state
    let initial_state: AppState = AppState {
        option: false,
        value: 0,
    };

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .delegate(Delegate)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::new(|data: &AppState, _: &_|format!("Current value: {}", data.value)))
        .with_default_spacer()
        .with_child(Label::new(|data: &AppState, _: &_|format!("IS selected: {}", data.option)))
        .center()
}

fn build_menu() -> Menu<AppState> {
    let menu = Menu::new("Druid Menu")
        .entry(
            MenuItem::new("Send Command")
                .command(COMMAND)
        )
        .separator()
        .entry(
            MenuItem::new("Change value")
                .on_activate(|_, data: &mut AppState, _|data.value = (data.value + 1) % 4)
        )
        .entry(
            MenuItem::new("1 Selected")
                .radio_item(1, Some(0))
                .lens(AppState::value)
        )
        .entry(
            MenuItem::new("2 Selected")
                .radio_item(2, Some(0))
                .lens(AppState::value)
        )
        .entry(
            // Implementing the radio item from hand
            MenuItem::new("3 Selected")
                .on_activate(|_, data: &mut AppState, _|if data.value == 3 {data.value = 0} else {data.value = 3})
                .selected_if(|data: &AppState, _|data.value == 3)
        )
        .separator()
        .entry(
            MenuItem::new("CheckBox")
                .toggle_data()
                .lens(AppState::option)
        )
        .entry(
            // Implementing the CheckBox from hand
            MenuItem::new("Manual CheckBox")
                .on_activate(|_, data: &mut AppState, _|data.option = !data.option)
                .selected_if(|data: &AppState, _|data.option)
        )
        .entry(
            MenuItem::new("Disabled")
                .on_activate(|_, _, _|panic!("disabled Menu Item was activated!"))
                .enabled(false)

        )
        .entry(
            MenuItem::new("Disabled Selectable")
                .on_activate(|_, _, _|panic!("disabled Menu Item was activated!"))
                .selected(false)
                .enabled(false)
        )
        //we dont add new menu items based on data!
        .rebuild_on(|_, _, _|false);

    Menu::empty()
        .entry(menu)

}

struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn command(&mut self, _: &mut DelegateCtx, _: Target, cmd: &Command, _: &mut AppState, _: &Env) -> Handled {
        if cmd.is(COMMAND) {
            println!("Clicked \"Send Command\"!");
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
