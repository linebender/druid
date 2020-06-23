// Copyright 2020 The Druid Authors.
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

use crate::widget::prelude::*;
use crate::Data;

/// A widget that draws semi-transparent rectangles of changing colors to help debug invalidation
/// regions.
pub struct DebugInvalidation<T, W> {
    inner: W,
    debug_color: u64,
    marker: std::marker::PhantomData<T>,
}

impl<T: Data, W: Widget<T>> DebugInvalidation<T, W> {
    /// Wraps a widget in a `DebugInvalidation`.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            debug_color: 0,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for DebugInvalidation<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);

        let color = env.get_debug_color(self.debug_color);
        let stroke_width = 2.0;
        let region = ctx.region().rects().to_owned();
        for rect in &region {
            let rect = rect.inset(-stroke_width / 2.0);
            ctx.stroke(rect, &color, stroke_width);
        }
        self.debug_color += 1;
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
