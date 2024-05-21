// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that switches dynamically between two child views.

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::{Data, Point, WidgetPod};
use tracing::instrument;

/// A widget that switches between two possible child views.
pub struct Either<T> {
    closure: Box<dyn Fn(&T, &Env) -> bool>,
    true_branch: WidgetPod<T, Box<dyn Widget<T>>>,
    false_branch: WidgetPod<T, Box<dyn Widget<T>>>,
    current: bool,
}

impl<T> Either<T> {
    /// Create a new widget that switches between two views.
    ///
    /// The given closure is evaluated on data change. If its value is `true`, then
    /// the `true_branch` widget is shown, otherwise `false_branch`.
    pub fn new(
        closure: impl Fn(&T, &Env) -> bool + 'static,
        true_branch: impl Widget<T> + 'static,
        false_branch: impl Widget<T> + 'static,
    ) -> Either<T> {
        Either {
            closure: Box::new(closure),
            true_branch: WidgetPod::new(true_branch).boxed(),
            false_branch: WidgetPod::new(false_branch).boxed(),
            current: false,
        }
    }
}

impl<T: Data> Widget<T> for Either<T> {
    #[instrument(name = "Either", level = "trace", skip(self, ctx, event, data, env), fields(branch = self.current))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if event.should_propagate_to_hidden() {
            self.true_branch.event(ctx, event, data, env);
            self.false_branch.event(ctx, event, data, env);
        } else {
            self.current_widget().event(ctx, event, data, env)
        }
    }

    #[instrument(name = "Either", level = "trace", skip(self, ctx, event, data, env), fields(branch = self.current))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.current = (self.closure)(data, env);
        }

        if event.should_propagate_to_hidden() {
            self.true_branch.lifecycle(ctx, event, data, env);
            self.false_branch.lifecycle(ctx, event, data, env);
        } else {
            self.current_widget().lifecycle(ctx, event, data, env)
        }
    }

    #[instrument(name = "Either", level = "trace", skip(self, ctx, _old_data, data, env), fields(branch = self.current))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let current = (self.closure)(data, env);
        if current != self.current {
            self.current = current;
            ctx.children_changed();
        }
        self.current_widget().update(ctx, data, env)
    }

    #[instrument(name = "Either", level = "trace", skip(self, ctx, bc, data, env), fields(branch = self.current))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let current_widget = self.current_widget();
        let size = current_widget.layout(ctx, bc, data, env);
        current_widget.set_origin(ctx, Point::ORIGIN);
        ctx.set_paint_insets(current_widget.paint_insets());
        size
    }

    #[instrument(name = "Either", level = "trace", skip(self, ctx, data, env), fields(branch = self.current))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.current_widget().paint(ctx, data, env)
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let current_widget = if self.current {
            &self.true_branch
        } else {
            &self.false_branch
        };
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![current_widget.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

impl<T> Either<T> {
    fn current_widget(&mut self) -> &mut WidgetPod<T, Box<dyn Widget<T>>> {
        if self.current {
            &mut self.true_branch
        } else {
            &mut self.false_branch
        }
    }
}
