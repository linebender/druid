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

//! Opening and closing windows and using window and context menus.

use druid::menu::{ContextMenu, MenuDesc, MenuItem};
use druid::widget::{Align, Button, Column, Label, Padding, Row};
use druid::{
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Event, EventCtx, LocalizedString,
    Selector, Widget, WindowDesc,
};

const MENU_COUNT_ACTION: Selector = Selector::new("menu-count-action");
const MENU_INCREMENT_ACTION: Selector = Selector::new("menu-increment-action");
const MENU_DECREMENT_ACTION: Selector = Selector::new("menu-decrement-action");

#[derive(Debug, Clone, Default, Data)]
struct State {
    menu_count: usize,
    selected: usize,
}

fn main() {
    simple_logger::init().unwrap();
    let main_window = WindowDesc::new(ui_builder).menu(make_menu(&State::default()));
    AppLauncher::with_window(main_window)
        .delegate(make_delegate())
        .launch(State::default())
        .expect("launch failed");
}

// this is just an experiment for how we might reduce boilerplate.
trait EventCtxExt {
    fn set_menu<T: 'static>(&mut self, menu: MenuDesc<T>);
}

impl EventCtxExt for EventCtx<'_, '_> {
    fn set_menu<T: 'static>(&mut self, menu: MenuDesc<T>) {
        let cmd = Command::new(druid::command::sys::SET_MENU, menu);
        self.submit_command(cmd, None);
    }
}

impl EventCtxExt for DelegateCtx<'_, '_> {
    fn set_menu<T: 'static>(&mut self, menu: MenuDesc<T>) {
        let cmd = Command::new(druid::command::sys::SET_MENU, menu);
        self.submit_command(cmd, None);
    }
}

fn ui_builder() -> impl Widget<State> {
    let text = LocalizedString::new("hello-counter")
        .with_arg("count", |data: &State, _env| data.menu_count.into());
    let label = Label::new(text);
    let inc_button = Button::<State>::new("Add menu item", |ctx, data, _env| {
        data.menu_count += 1;
        ctx.set_menu(make_menu::<State>(data));
    });
    let dec_button = Button::<State>::new("Remove menu item", |ctx, data, _env| {
        data.menu_count = data.menu_count.saturating_sub(1);
        ctx.set_menu(make_menu::<State>(data));
    });

    let mut col = Column::new();
    col.add_child(Align::centered(Padding::new(5.0, label)), 1.0);
    let mut row = Row::new();
    row.add_child(Padding::new(5.0, inc_button), 1.0);
    row.add_child(Padding::new(5.0, dec_button), 1.0);
    col.add_child(row, 1.0);
    col
}

fn make_delegate() -> AppDelegate<State> {
    AppDelegate::new().event_handler(|event, data, _env, ctx| {
        match event {
            Event::Command(ref cmd) if cmd.selector == druid::command::sys::NEW_FILE => {
                let new_win = WindowDesc::new(ui_builder).menu(make_menu(data));
                let command = Command::new(druid::command::sys::NEW_WINDOW, new_win);
                ctx.submit_command(command, None);
                None
            }
            Event::Command(ref cmd) if cmd.selector == MENU_COUNT_ACTION => {
                data.selected = *cmd.get_object().unwrap();
                ctx.set_menu(make_menu::<State>(data));
                None
            }
            // wouldn't it be nice if a menu (like a button) could just mutate state
            // directly if desired?
            Event::Command(ref cmd) if cmd.selector == MENU_INCREMENT_ACTION => {
                data.menu_count += 1;
                ctx.set_menu(make_menu::<State>(data));
                None
            }
            Event::Command(ref cmd) if cmd.selector == MENU_DECREMENT_ACTION => {
                data.menu_count = data.menu_count.saturating_sub(1);
                ctx.set_menu(make_menu::<State>(data));
                None
            }
            Event::MouseDown(ref mouse) if mouse.button.is_right() => {
                let menu = ContextMenu::new(make_context_menu::<State>(), mouse.pos);
                let cmd = Command::new(druid::command::sys::SHOW_CONTEXT_MENU, menu);
                ctx.submit_command(cmd, None);
                None
            }
            other => Some(other),
        }
    })
}

#[allow(unused_assignments)]
fn make_menu<T: Data>(state: &State) -> MenuDesc<T> {
    let mut base = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        base = druid::menu::sys::mac::menu_bar();
    }
    #[cfg(target_os = "windows")]
    {
        base = base.append(druid::menu::sys::win::file::default());
    }
    if state.menu_count != 0 {
        base = base.append(
            MenuDesc::new(LocalizedString::new("Custom")).append_iter(|| {
                (0..state.menu_count).map(|i| {
                    MenuItem::new(
                        LocalizedString::new("hello-counter")
                            .with_arg("count", move |_, _| i.into()),
                        Command::new(MENU_COUNT_ACTION, i),
                    )
                    .disabled_if(|| i % 3 == 0)
                    .selected_if(|| i == state.selected)
                })
            }),
        );
    }
    base
}

fn make_context_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::empty()
        .append(MenuItem::new(
            LocalizedString::new("Increment"),
            MENU_INCREMENT_ACTION,
        ))
        .append(MenuItem::new(
            LocalizedString::new("Decrement"),
            MENU_DECREMENT_ACTION,
        ))
}
