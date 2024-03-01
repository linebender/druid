// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A [`Widget`] that uses a [`Lens`] to change the [`Data`] of its child.

use std::marker::PhantomData;

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::widget::WidgetWrapper;
use crate::{Data, Lens};

use tracing::{instrument, trace};

/// A wrapper for its widget subtree to have access to a part
/// of its parent's data.
///
/// Every widget in Druid is instantiated with access to data of some
/// type; the root widget has access to the entire application data.
/// Often, a part of the widget hierarchy is only concerned with a part
/// of that data. The `LensWrap` widget is a way to "focus" the data
/// reference down, for the subtree. One advantage is performance;
/// data changes that don't intersect the scope of the lens aren't
/// propagated.
///
/// Another advantage is generality and reuse. If a widget (or tree of
/// widgets) is designed to work with some chunk of data, then with a
/// lens that same code can easily be reused across all occurrences of
/// that chunk within the application state.
///
/// This wrapper takes a [`Lens`] as an argument, which is a specification
/// of a struct field, or some other way of narrowing the scope.
pub struct LensWrap<T, U, L, W> {
    child: W,
    lens: L,
    // The following is a workaround for otherwise getting E0207.
    // the 'in' data type of the lens
    phantom_u: PhantomData<U>,
    // the 'out' data type of the lens
    phantom_t: PhantomData<T>,
}

impl<T, U, L, W> LensWrap<T, U, L, W> {
    /// Wrap a widget with a lens.
    ///
    /// When the lens has type `Lens<T, U>`, the child widget has data
    /// of type `U`, and the wrapped widget has data of type `T`.
    pub fn new(child: W, lens: L) -> LensWrap<T, U, L, W> {
        LensWrap {
            child,
            lens,
            phantom_u: Default::default(),
            phantom_t: Default::default(),
        }
    }

    /// Get a reference to the lens.
    pub fn lens(&self) -> &L {
        &self.lens
    }

    /// Get a mutable reference to the lens.
    pub fn lens_mut(&mut self) -> &mut L {
        &mut self.lens
    }
}

impl<T, U, L, W> Widget<T> for LensWrap<T, U, L, W>
where
    T: Data,
    U: Data,
    L: Lens<T, U>,
    W: Widget<U>,
{
    #[instrument(name = "LensWrap", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let child = &mut self.child;
        self.lens
            .with_mut(data, |data| child.event(ctx, event, data, env))
    }

    #[instrument(name = "LensWrap", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let child = &mut self.child;
        self.lens
            .with(data, |data| child.lifecycle(ctx, event, data, env))
    }

    #[instrument(
        name = "LensWrap",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        let child = &mut self.child;
        let lens = &self.lens;
        lens.with(old_data, |old_data| {
            lens.with(data, |data| {
                if ctx.has_requested_update() || !old_data.same(data) || ctx.env_changed() {
                    child.update(ctx, old_data, data, env);
                } else {
                    trace!("skipping child update");
                }
            })
        })
    }

    #[instrument(name = "LensWrap", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let child = &mut self.child;
        self.lens
            .with(data, |data| child.layout(ctx, bc, data, env))
    }

    #[instrument(name = "LensWrap", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let child = &mut self.child;
        self.lens.with(data, |data| child.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.child.id()
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let child_state = self.lens.with(data, |data| self.child.debug_state(data));
        DebugState {
            display_name: "LensWrap".to_string(),
            children: vec![child_state],
            ..Default::default()
        }
    }
}

impl<T, U, L, W> WidgetWrapper for LensWrap<T, U, L, W> {
    widget_wrapper_body!(W, child);
}
