// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::Data;
use tracing::instrument;

/// A widget that draws semi-transparent rectangles of changing colors to help debug invalidation
/// regions.
pub struct DebugInvalidation<T, W> {
    child: W,
    debug_color: u64,
    marker: std::marker::PhantomData<T>,
}

impl<T: Data, W: Widget<T>> DebugInvalidation<T, W> {
    /// Wraps a widget in a `DebugInvalidation`.
    pub fn new(child: W) -> Self {
        Self {
            child,
            debug_color: 0,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for DebugInvalidation<T, W> {
    #[instrument(
        name = "DebugInvalidation",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    #[instrument(
        name = "DebugInvalidation",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    #[instrument(
        name = "DebugInvalidation",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    #[instrument(
        name = "DebugInvalidation",
        level = "trace",
        skip(self, ctx, bc, data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.child.layout(ctx, bc, data, env)
    }

    #[instrument(
        name = "DebugInvalidation",
        level = "trace",
        skip(self, ctx, data, env)
    )]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);

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
        self.child.id()
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.debug_state(data)],
            ..Default::default()
        }
    }
}
