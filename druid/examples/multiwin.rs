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

//! Opening and closing windows and using window and context menus.

use druid::widget::prelude::*;
use druid::widget::{
    Align, BackgroundBrush, Button, Controller, ControllerHost, Flex, Label, Padding,
};
use druid::Target::Global;
use druid::{
    commands as sys_cmds, AppDelegate, AppLauncher, Application, Color, Command, ContextMenu, Data,
    DelegateCtx, Handled, LocalizedString, MenuDesc, MenuItem, Selector, Target, WindowDesc,
    WindowId,
};
use log::info;

const MENU_COUNT_ACTION: Selector<usize> = Selector::new("menu-count-action");
const MENU_INCREMENT_ACTION: Selector = Selector::new("menu-increment-action");
const MENU_DECREMENT_ACTION: Selector = Selector::new("menu-decrement-action");
const MENU_SWITCH_GLOW_ACTION: Selector = Selector::new("menu-switch-glow");

#[derive(Debug, Clone, Default, Data)]
struct State {
    menu_count: usize,
    selected: usize,
    glow_hot: bool,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .menu(make_menu(&State::default()))
        .title(
            LocalizedString::new("multiwin-demo-window-title").with_placeholder("Many windows!"),
        );
    AppLauncher::with_window(main_window)
        .delegate(Delegate {
            windows: Vec::new(),
        })
        .use_simple_logger()
        .launch(State::default())
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<State> {
    let text = LocalizedString::new("hello-counter")
        .with_arg("count", |data: &State, _env| data.menu_count.into());
    let label = Label::new(text);
    let inc_button = Button::<State>::new("Add menu item")
        .on_click(|ctx, _data, _env| ctx.submit_command(MENU_INCREMENT_ACTION.to(Global)));
    let dec_button = Button::<State>::new("Remove menu item")
        .on_click(|ctx, _data, _env| ctx.submit_command(MENU_DECREMENT_ACTION.to(Global)));
    let new_button = Button::<State>::new("New window").on_click(|ctx, _data, _env| {
        ctx.submit_command(sys_cmds::NEW_FILE.to(Global));
    });
    let quit_button = Button::<State>::new("Quit app").on_click(|_ctx, _data, _env| {
        Application::global().quit();
    });

    let mut col = Flex::column();
    col.add_flex_child(Align::centered(Padding::new(5.0, label)), 1.0);
    let mut row = Flex::row();
    row.add_child(Padding::new(5.0, inc_button));
    row.add_child(Padding::new(5.0, dec_button));
    col.add_flex_child(Align::centered(row), 1.0);
    let mut row = Flex::row();
    row.add_child(Padding::new(5.0, new_button));
    row.add_child(Padding::new(5.0, quit_button));
    col.add_flex_child(Align::centered(row), 1.0);
    let content = ControllerHost::new(col, ContextMenuController);
    Glow::new(content)
}

struct Glow<W> {
    inner: W,
}

impl<W> Glow<W> {
    pub fn new(inner: W) -> Glow<W> {
        Glow { inner }
    }
}

impl<W: Widget<State>> Widget<State> for Glow<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut State, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &State, env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &State, data: &State, env: &Env) {
        if old_data.glow_hot != data.glow_hot {
            ctx.request_paint();
        }
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &State,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &State, env: &Env) {
        if data.glow_hot && ctx.is_hot() {
            BackgroundBrush::Color(Color::rgb8(200, 55, 55)).paint(ctx, data, env);
        }
        self.inner.paint(ctx, data, env);
    }
}

struct ContextMenuController;
struct Delegate {
    windows: Vec<WindowId>,
}

impl<T, W: Widget<T>> Controller<T, W> for ContextMenuController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(ref mouse) if mouse.button.is_right() => {
                let menu = ContextMenu::new(make_context_menu::<State>(), mouse.pos);
                ctx.show_context_menu(menu);
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

impl AppDelegate<State> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut State,
        _env: &Env,
    ) -> Handled {
        match cmd {
            _ if cmd.is(sys_cmds::NEW_FILE) => {
                let new_win = WindowDesc::new(ui_builder)
                    .menu(make_menu(data))
                    .window_size((data.selected as f64 * 100.0 + 300.0, 500.0));
                ctx.new_window(new_win);
                Handled::Yes
            }
            _ if cmd.is(MENU_COUNT_ACTION) => {
                data.selected = *cmd.get_unchecked(MENU_COUNT_ACTION);
                let menu = make_menu::<State>(data);
                for id in &self.windows {
                    ctx.set_menu(menu.clone(), *id);
                }
                Handled::Yes
            }
            // wouldn't it be nice if a menu (like a button) could just mutate state
            // directly if desired?
            _ if cmd.is(MENU_INCREMENT_ACTION) => {
                data.menu_count += 1;
                let menu = make_menu::<State>(data);
                for id in &self.windows {
                    ctx.set_menu(menu.clone(), *id);
                }
                Handled::Yes
            }
            _ if cmd.is(MENU_DECREMENT_ACTION) => {
                data.menu_count = data.menu_count.saturating_sub(1);
                let menu = make_menu::<State>(data);
                for id in &self.windows {
                    ctx.set_menu(menu.clone(), *id);
                }
                Handled::Yes
            }
            _ if cmd.is(MENU_SWITCH_GLOW_ACTION) => {
                data.glow_hot = !data.glow_hot;
                Handled::Yes
            }
            _ => Handled::No,
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
        self.windows.push(id);
    }

    fn window_removed(
        &mut self,
        id: WindowId,
        _data: &mut State,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        info!("Window removed, id: {:?}", id);
        if let Some(pos) = self.windows.iter().position(|x| *x == id) {
            self.windows.remove(pos);
        }
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
                        MENU_COUNT_ACTION.with(i),
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
        .append(MenuItem::new(
            LocalizedString::new("Glow when hot"),
            MENU_SWITCH_GLOW_ACTION,
        ))
}
