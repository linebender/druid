// Copyright 2020 The xi-editor Authors.
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

//! Helper types for test writing.
//!
//! This includes tools for making throwaway widgets more easily.

use crate::widget::WidgetExt;
use crate::*;

pub type EventFn<S, T> = dyn FnMut(&mut S, &mut EventCtx, &Event, &T, &Env);
pub type LifeCycleFn<S, T> = dyn FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle, &T, &Env);
pub type UpdateFn<S, T> = dyn FnMut(&mut S, &mut UpdateCtx, &T, &T, &Env);
pub type LayoutFn<S, T> = dyn FnMut(&mut S, &mut LayoutCtx, &BoxConstraints, &T, &Env) -> Size;
pub type PaintFn<S, T> = dyn FnMut(&mut S, &mut PaintCtx, &T, &Env);

pub const REPLACE_CHILD: Selector = Selector::new("druid-test.replace-child");

/// A widget that can be constructed from individual functions, builder-style.
///
/// This widget is generic over its state, which is passed in at construction time.
pub struct ModularWidget<S, T> {
    state: S,
    event: Option<Box<EventFn<S, T>>>,
    lifecycle: Option<Box<LifeCycleFn<S, T>>>,
    update: Option<Box<UpdateFn<S, T>>>,
    layout: Option<Box<LayoutFn<S, T>>>,
    paint: Option<Box<PaintFn<S, T>>>,
}

/// A widget that can replace its child on command
pub struct ReplaceChild<T: Data> {
    inner: WidgetPod<T, Box<dyn Widget<T>>>,
    replacer: Box<dyn Fn() -> Box<dyn Widget<T>>>,
}

#[allow(dead_code)]
impl<S, T> ModularWidget<S, T> {
    pub fn new(state: S) -> Self {
        ModularWidget {
            state,
            event: None,
            lifecycle: None,
            update: None,
            layout: None,
            paint: None,
        }
    }

    pub fn event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx, &Event, &T, &Env) + 'static,
    ) -> Self {
        self.event = Some(Box::new(f));
        self
    }

    pub fn lifecycle_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle, &T, &Env) + 'static,
    ) -> Self {
        self.lifecycle = Some(Box::new(f));
        self
    }

    pub fn update_fn(
        mut self,
        f: impl FnMut(&mut S, &mut UpdateCtx, &T, &T, &Env) + 'static,
    ) -> Self {
        self.update = Some(Box::new(f));
        self
    }

    pub fn layout_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LayoutCtx, &BoxConstraints, &T, &Env) -> Size + 'static,
    ) -> Self {
        self.layout = Some(Box::new(f));
        self
    }

    pub fn paint_fn(mut self, f: impl FnMut(&mut S, &mut PaintCtx, &T, &Env) + 'static) -> Self {
        self.paint = Some(Box::new(f));
        self
    }
}

impl<S, T: Data> Widget<T> for ModularWidget<S, T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(f) = self.event.as_mut() {
            f(&mut self.state, ctx, event, data, env)
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let Some(f) = self.lifecycle.as_mut() {
            f(&mut self.state, ctx, event, data, env)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(f) = self.update.as_mut() {
            f(&mut self.state, ctx, old_data, data, env)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let ModularWidget {
            ref mut state,
            ref mut layout,
            ..
        } = self;
        layout
            .as_mut()
            .map(|f| f(state, ctx, bc, data, env))
            .unwrap_or(Size::new(100., 100.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(f) = self.paint.as_mut() {
            f(&mut self.state, ctx, data, env)
        }
    }
}

impl<T: Data> ReplaceChild<T> {
    pub fn new<W: Widget<T> + 'static>(
        inner: impl Widget<T> + 'static,
        f: impl Fn() -> W + 'static,
    ) -> Self {
        let inner = WidgetPod::new(inner.boxed());
        let replacer = Box::new(move || f().boxed());
        ReplaceChild { inner, replacer }
    }
}

impl<T: Data> Widget<T> for ReplaceChild<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Command(cmd) = event {
            if cmd.selector == REPLACE_CHILD {
                self.inner = WidgetPod::new((self.replacer)());
                ctx.children_changed();
                return;
            }
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env)
    }
}
