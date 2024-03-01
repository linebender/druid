// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that provides an explicit identity to a child.

use crate::debug_state::DebugState;
use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::widget::WidgetWrapper;
use crate::Data;
use tracing::instrument;

/// A wrapper that adds an identity to an otherwise anonymous widget.
pub struct IdentityWrapper<W> {
    id: WidgetId,
    child: W,
}

impl<W> IdentityWrapper<W> {
    /// Assign an identity to a widget.
    pub fn wrap(child: W, id: WidgetId) -> IdentityWrapper<W> {
        IdentityWrapper { id, child }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for IdentityWrapper<W> {
    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, bc, data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.child.layout(ctx, bc, data, env)
    }

    #[instrument(name = "IdentityWrapper", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.debug_state(data)],
            ..Default::default()
        }
    }
}

impl<W> WidgetWrapper for IdentityWrapper<W> {
    widget_wrapper_body!(W, child);
}
