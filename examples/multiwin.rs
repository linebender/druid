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
use druid::widget::{ActionWrapper, Align, Button, Column, Label, Padding};
use druid::{
    Action, AppLauncher, BaseState, BoxConstraints, Command, Data, Env, Event, EventCtx, HotKey,
    LayoutCtx, LocalizedString, PaintCtx, Selector, SysMods, UpdateCtx, Widget, WindowDesc,
};

fn main() {
    simple_logger::init().unwrap();
    let main_window = WindowDesc::new(ui_builder);
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<u32> {
    let text =
        LocalizedString::new("hello-counter").with_arg("count", |data: &u32, _env| (*data).into());
    let label = Label::new(text);
    let button = Button::new("increment");

    let mut col = Column::new();
    col.add_child(Align::centered(Padding::uniform(5.0, label)), 1.0);
    col.add_child(Padding::uniform(5.0, button), 1.0);
    let wrapper = ActionWrapper::new(col, |data: &mut u32, _env| *data += 1);

    EventInterceptor::new(wrapper, |event, ctx, _data, _env| {
        if let Event::KeyDown(e) = event {
            if HotKey::new(SysMods::Cmd, "n").matches(e) {
                eprintln!("cmd-N");
                let new_win = WindowDesc::new(ui_builder);
                let command = Command::new(Selector::NEW_WINDOW, new_win);
                ctx.submit_command(command, None);
                return None;
            }
            if HotKey::new(SysMods::Cmd, "w").matches(e) {
                eprintln!("cmd-W");
                let id = ctx.window_id();
                let command = Command::new(Selector::CLOSE_WINDOW, id);
                ctx.submit_command(command, None);
                return None;
            }
        }
        Some(event)
    })
}

// should something like this be in druid proper? I'm just experimenting here...
/// A widget that wraps another widget and intercepts the `event` fn.
///
/// This is instantiated with a closure that has the same signature as `event`,
/// and which can either consume events itself or return them to have them
/// be passed to the inner widget.
struct EventInterceptor<T> {
    inner: Box<dyn Widget<T> + 'static>,
    f: Box<dyn Fn(Event, &mut EventCtx, &mut T, &Env) -> Option<Event>>,
}

impl<T: Data + 'static> EventInterceptor<T> {
    fn new<W, F>(inner: W, f: F) -> Self
    where
        W: Widget<T> + 'static,
        F: Fn(Event, &mut EventCtx, &mut T, &Env) -> Option<Event> + 'static,
    {
        EventInterceptor {
            inner: Box::new(inner),
            f: Box::new(f),
        }
    }
}

impl<T: Data> Widget<T> for EventInterceptor<T> {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, d: &T, env: &Env) {
        self.inner.paint(ctx, state, d, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, d: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, d, env)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        if !ctx.has_focus() {
            ctx.request_focus();
        }
        let event = event.clone();
        let event = (self.f)(event, ctx, data, env);
        match event {
            Some(event) => self.inner.event(&event, ctx, data, env),
            None => None,
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: Option<&T>, new: &T, env: &Env) {
        self.inner.update(ctx, old, new, env)
    }
}
