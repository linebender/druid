// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::debug_state::DebugState;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Size, UpdateCtx, Widget, WidgetPod,
};

/// A widget wrapper which disables the child widget if the provided closure return true.
///
/// See [`is_disabled`] or [`set_disabled`] for more info about disabled state.
///
/// [`is_disabled`]: crate::EventCtx::is_disabled
/// [`set_disabled`]: crate::EventCtx::set_disabled
pub struct DisabledIf<T, W> {
    child: WidgetPod<T, W>,
    disabled_if: Box<dyn Fn(&T, &Env) -> bool>,
}

impl<T: Data, W: Widget<T>> DisabledIf<T, W> {
    /// Creates a new `DisabledIf` widget with the child widget and the closure to decide if the
    /// widget should be [`disabled`].
    ///
    /// [`disabled`]: crate::EventCtx::is_disabled
    pub fn new(widget: W, disabled_if: impl Fn(&T, &Env) -> bool + 'static) -> Self {
        DisabledIf {
            child: WidgetPod::new(widget),
            disabled_if: Box::new(disabled_if),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for DisabledIf<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.set_disabled((self.disabled_if)(data, env));
        }
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        ctx.set_disabled((self.disabled_if)(data, env));
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.child.set_origin(ctx, Point::ZERO);
        ctx.set_baseline_offset(self.child.baseline_offset());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.widget().debug_state(data)],
            ..Default::default()
        }
    }
}
