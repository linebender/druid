// Copyright 2018 Google LLC
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

//! Sample GUI app.

extern crate xi_win_shell;
extern crate xi_win_ui;
extern crate direct2d;
extern crate directwrite;

use xi_win_shell::menu::Menu;
use xi_win_shell::win_main;
use xi_win_shell::window::WindowBuilder;

use xi_win_ui::{GuiMain, GuiState};
use xi_win_ui::{Button, FooWidget, Padding, Row};
use xi_win_ui::COMMAND_EXIT;

fn main() {
    xi_win_shell::init();

    let mut file_menu = Menu::new();
    file_menu.add_item(COMMAND_EXIT, "E&xit");
    let mut menubar = Menu::new();
    menubar.add_dropdown(file_menu, "&File");

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = GuiState::new();
    let foo1 = state.add(FooWidget, &[]);
    let foo1 = state.add(Padding::uniform(10.0), &[foo1]);
    let foo2 = state.add(FooWidget, &[]);
    let foo2 = state.add(Padding::uniform(10.0), &[foo2]);
    let button = state.add(Button::new("Press me"), &[]);
    let button2 = state.add(Button::new("Don't press me"), &[]);
    let root = state.add(Row::default(), &[foo1, foo2, button, button2]);
    state.set_root(root);
    state.add_listener(button, move |_: bool, ctx| {
        ctx.poke(button2, &mut "You clicked it!".to_string());
    });
    state.add_listener(button2, move |_: bool, ctx| {
        ctx.poke(button2, &mut "Naughty naughty".to_string());
    });
    builder.set_handler(Box::new(GuiMain::new(state)));
    builder.set_title("Hello example");
    builder.set_menu(menubar);
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
