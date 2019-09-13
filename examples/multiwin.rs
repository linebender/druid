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

//! Manually opening and closing windows.

use druid::kurbo::Size;
use druid::menu::{Menu, MenuItem};
use druid::widget::{Align, Button, Column, Label, Padding};
use druid::{
    AppLauncher, BaseState, BoxConstraints, Command, Data, Env, Event, EventCtx, HotKey, KeyCode,
    LayoutCtx, LocalizedString, PaintCtx, Selector, SysMods, UpdateCtx, Widget, WindowDesc,
};

const MENU_COUNT_ACTION: Selector = Selector::new("menu-count-action");

#[derive(Debug, Clone, Default)]
struct State {
    menu_count: usize,
    selected: usize,
}

fn main() {
    simple_logger::init().unwrap();
    let main_window = WindowDesc::new(ui_builder).menu(|_, _| make_menu(State::default()));
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<u32> {
    let text =
        LocalizedString::new("hello-counter").with_arg("count", |data: &u32, _env| (*data).into());
    let label = Label::new(text);
    let button = Button::new("increment", |_ctx, data: &mut u32, _env| *data += 1);

    let mut col = Column::new();
    col.add_child(Align::centered(Padding::uniform(5.0, label)), 1.0);
    col.add_child(Padding::uniform(5.0, button), 1.0);

    EventInterceptor::new(
        State::default(),
        col,
        |event, ctx, state, _data, _env| match event {
            Event::KeyUp(key) if HotKey::new(None, KeyCode::ArrowUp).matches(key) => {
                eprintln!("{:?}", key);
                state.menu_count += 1;
                ctx.submit_command(
                    Command::new(Selector::SET_MENU, make_menu::<u32>(state.clone())),
                    None,
                );
                eprintln!("count {}", state.menu_count);
                None
            }
            Event::KeyUp(key) if HotKey::new(None, KeyCode::ArrowDown).matches(key) => {
                state.menu_count = state.menu_count.saturating_sub(1);
                ctx.submit_command(
                    Command::new(Selector::SET_MENU, make_menu::<u32>(state.clone())),
                    None,
                );
                eprintln!("count {}", state.menu_count);
                None
            }
            Event::KeyUp(key) if HotKey::new(SysMods::Cmd, "n").matches(key) => {
                eprintln!("cmd-N");
                let new_win = WindowDesc::new(ui_builder);
                let command = Command::new(Selector::NEW_WINDOW, new_win);
                ctx.submit_command(command, None);
                None
            }
            Event::KeyUp(key) if HotKey::new(SysMods::Cmd, "w").matches(key) => {
                eprintln!("cmd-W");
                let id = ctx.window_id();
                let command = Command::new(Selector::CLOSE_WINDOW, id);
                ctx.submit_command(command, None);
                None
            }
            Event::Command(ref cmd) if &cmd.selector == &MENU_COUNT_ACTION => {
                state.selected = *cmd.get_object().unwrap();
                eprintln!("{}", state.selected);
                ctx.submit_command(
                    Command::new(Selector::SET_MENU, make_menu::<u32>(state.clone())),
                    None,
                );
                None
            }
            other => Some(other),
        },
    )
}

// should something like this be in druid proper? I'm just experimenting here...
/// A widget that wraps another widget and intercepts the `event` fn.
///
/// This is instantiated with a closure that has the same signature as `event`,
/// and which can either consume events itself or return them to have them
/// be passed to the inner widget.
struct EventInterceptor<T, S> {
    /// Custom widget-level state
    state: S,
    inner: Box<dyn Widget<T> + 'static>,
    f: Box<dyn Fn(Event, &mut EventCtx, &mut S, &mut T, &Env) -> Option<Event>>,
}

impl<T: Data + 'static, S> EventInterceptor<T, S> {
    fn new<W, F>(state: S, inner: W, f: F) -> Self
    where
        W: Widget<T> + 'static,
        F: Fn(Event, &mut EventCtx, &mut S, &mut T, &Env) -> Option<Event> + 'static,
    {
        EventInterceptor {
            state,
            inner: Box::new(inner),
            f: Box::new(f),
        }
    }
}

impl<T: Data, S> Widget<T> for EventInterceptor<T, S> {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, d: &T, env: &Env) {
        self.inner.paint(ctx, state, d, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, d: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, d, env)
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut T, env: &Env) {
        if !ctx.has_focus() {
            ctx.request_focus();
        }
        let event = event.clone();
        let EventInterceptor { state, inner, f } = self;
        if let Some(event) = (f)(event, ctx, state, data, env) {
            inner.event(&event, ctx, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: Option<&T>, new: &T, env: &Env) {
        self.inner.update(ctx, old, new, env)
    }
}

fn make_menu<T: Data>(state: State) -> Menu<T> {
    druid::menu::macos_menu_bar().append(Menu::new(LocalizedString::new("custom")).append_iter(
        || {
            (0..state.menu_count).map(|i| {
                MenuItem::new(
                    LocalizedString::new("hello-counter").with_arg("count", move |_, _| i.into()),
                    Command::new(MENU_COUNT_ACTION, i),
                )
                .disabled_if(|| i % 3 == 0)
                .selected_if(|| i == state.selected)
            })
        },
    ))
}
