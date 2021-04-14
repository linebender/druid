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

use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Size, UpdateCtx, Widget, WidgetPod,
};

/// A widget wrapper which disables the inner widget if the provided closure return true.
///
/// See [`is_disabled`] or [`set_disabled`] for more info about disabled state.
///
/// [`is_disabled`]: crate::EventCtx::is_disabled
/// [`set_disabled`]: crate::EventCtx::set_disabled
pub struct DisabledIf<T, W> {
    inner: WidgetPod<T, W>,
    disabled_if: Box<dyn Fn(&T, &Env) -> bool>,
}

impl<T: Data, W: Widget<T>> DisabledIf<T, W> {
    /// Creates a new `DisabledIf` widget with the inner widget and the closure to decide if the
    /// widget should be [`disabled`].
    ///
    /// [`disabled`]: crate::EventCtx::is_disabled
    pub fn new(widget: W, disabled_if: impl Fn(&T, &Env) -> bool + 'static) -> Self {
        DisabledIf {
            inner: WidgetPod::new(widget),
            disabled_if: Box::new(disabled_if),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for DisabledIf<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.set_disabled((self.disabled_if)(data, env));
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        ctx.set_disabled((self.disabled_if)(data, env));
        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_origin(ctx, data, env, Point::ZERO);
        ctx.set_baseline_offset(self.inner.baseline_offset());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
    }
}
