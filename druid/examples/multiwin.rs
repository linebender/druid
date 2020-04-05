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

use druid::widget::{Align, Button, Flex, Label, Padding};
use druid::{
    commands as sys_cmds, AppDelegate, AppLauncher, Command, ContextMenu, Data, DelegateCtx, Env,
    Event, EventCtx, LocalizedString, MenuDesc, MenuItem, Selector, Target, Widget, WindowDesc,
    WindowId,
};

use log::info;

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
    let main_window = WindowDesc::new(ui_builder)
        .menu(make_menu(&State::default()))
        .title(
            LocalizedString::new("multiwin-demo-window-title").with_placeholder("Many windows!"),
        );
    AppLauncher::with_window(main_window)
        .delegate(Delegate)
        .launch(State::default())
        .expect("launch failed");
}

// this is just an experiment for how we might reduce boilerplate.
trait EventCtxExt {
    fn set_menu<T: 'static>(&mut self, menu: MenuDesc<T>);
}

impl EventCtxExt for EventCtx<'_> {
    fn set_menu<T: 'static>(&mut self, menu: MenuDesc<T>) {
        let cmd = Command::new(druid::commands::SET_MENU, menu);
        let target = self.window_id();
        self.submit_command(cmd, target);
    }
}

fn ui_builder() -> impl Widget<State> {
    let text = LocalizedString::new("hello-counter")
        .with_arg("count", |data: &State, _env| data.menu_count.into());
    let label = Label::new(text);
    let inc_button = Button::<State>::new("Add menu item").on_click(|ctx, data, _env| {
        data.menu_count += 1;
        ctx.set_menu(make_menu::<State>(data));
    });
    let dec_button = Button::<State>::new("Remove menu item").on_click(|ctx, data, _env| {
        data.menu_count = data.menu_count.saturating_sub(1);
        ctx.set_menu(make_menu::<State>(data));
    });

    let mut col = Flex::column();
    col.add_flex_child(Align::centered(Padding::new(5.0, label)), 1.0);
    let mut row = Flex::row();
    row.add_child(Padding::new(5.0, inc_button));
    row.add_child(Padding::new(5.0, dec_button));
    col.add_flex_child(Align::centered(row), 1.0);
    col
}

struct Delegate;

impl AppDelegate<State> for Delegate {
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        window_id: WindowId,
        event: Event,
        _data: &mut State,
        _env: &Env,
    ) -> Option<Event> {
        match event {
            Event::MouseDown(ref mouse) if mouse.button.is_right() => {
                let menu = ContextMenu::new(make_context_menu::<State>(), mouse.pos);
                let cmd = Command::new(druid::commands::SHOW_CONTEXT_MENU, menu);
                ctx.submit_command(cmd, Target::Window(window_id));
                None
            }
            other => Some(other),
        }
    }

    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: &Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> bool {
        match (target, &cmd.selector) {
            (_, &sys_cmds::NEW_FILE) => {
                let new_win = WindowDesc::new(ui_builder)
                    .menu(make_menu(data))
                    .window_size((data.selected as f64 * 100.0 + 300.0, 500.0));
                let command = Command::one_shot(sys_cmds::NEW_WINDOW, new_win);
                ctx.submit_command(command, Target::Global);
                false
            }
            (Target::Window(id), &MENU_COUNT_ACTION) => {
                data.selected = *cmd.get_object().unwrap();
                let menu = make_menu::<State>(data);
                let cmd = Command::new(druid::commands::SET_MENU, menu);
                ctx.submit_command(cmd, *id);
                false
            }
            // wouldn't it be nice if a menu (like a button) could just mutate state
            // directly if desired?
            (Target::Window(id), &MENU_INCREMENT_ACTION) => {
                data.menu_count = data.menu_count + 1;
                let menu = make_menu::<State>(data);
                let cmd = Command::new(druid::commands::SET_MENU, menu);
                ctx.submit_command(cmd, *id);
                false
            }
            (Target::Window(id), &MENU_DECREMENT_ACTION) => {
                data.menu_count = data.menu_count.saturating_sub(1);
                let menu = make_menu::<State>(data);
                let cmd = Command::new(druid::commands::SET_MENU, menu);
                ctx.submit_command(cmd, *id);
                false
            }
            _ => true,
        }
    }

    fn window_added(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        info!("Window added, id: {:?}", id);
    }
    fn window_removed(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        info!("Window removed, id: {:?}", id);
    }
}

#[allow(unused_assignments)]
fn make_menu<T: Data>(state: &State) -> MenuDesc<T> {
    let mut base = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        base = druid::platform_menus::mac::menu_bar();
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        base = base.append(druid::platform_menus::win::file::default());
    }
    if state.menu_count != 0 {
        base = base.append(
            MenuDesc::new(LocalizedString::new("Custom")).append_iter(|| {
                (1..state.menu_count + 1).map(|i| {
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
