// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget-controlling widget.

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::widget::{Axis, WidgetWrapper};

/// A trait for types that modify behaviour of a child widget.
///
/// A `Controller` is a type that manages a child widget, overriding or
/// customizing its event handling or update behaviour.
///
/// A controller can only handle events and update; it cannot effect layout
/// or paint.
///
/// `Controller` is a convenience; anything it can do could also be done
/// by creating a custom [`Widget`] that owned a child. This is somewhat cumbersome,
/// however, especially when you only want to intercept or modify one or two events.
///
/// The methods on `Controller` are identical to the methods on [`Widget`],
/// except that they are also passed the controller's child. The controller
/// is responsible for **explicitly** forwarding calls on to the child as needed.
///
/// A `Controller` is used with a [`ControllerHost`], which manages the relationship
/// between it and its child; although in general you would use the
/// [`WidgetExt::controller`] method instead of instantiating a host directly.
///
/// # Examples
///
/// A [`TextBox`] that takes focus on launch:
///
/// ```
/// # use druid::widget::{Controller, TextBox};
/// # use druid::{Env, Event, EventCtx, Widget};
/// struct TakeFocus;
///
/// impl<T, W: Widget<T>> Controller<T, W> for TakeFocus {
///     fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
///         if let Event::WindowConnected = event {
///             ctx.request_focus();
///         }
///         child.event(ctx, event, data, env)
///     }
/// }
/// ```
///
/// [`TextBox`]: super::TextBox
/// [`WidgetExt::controller`]: super::WidgetExt::controller
pub trait Controller<T, W: Widget<T>> {
    /// Analogous to [`Widget::event`].
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        child.event(ctx, event, data, env)
    }

    /// Analogous to [`Widget::lifecycle`].
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        child.lifecycle(ctx, event, data, env)
    }

    /// Analogous to [`Widget::update`].
    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        child.update(ctx, old_data, data, env)
    }
}

/// A [`Widget`] that manages a child and a [`Controller`].
pub struct ControllerHost<W, C> {
    widget: W,
    controller: C,
}

impl<W, C> ControllerHost<W, C> {
    /// Create a new `ControllerHost`.
    pub fn new(widget: W, controller: C) -> ControllerHost<W, C> {
        ControllerHost { widget, controller }
    }
}

impl<T, W: Widget<T>, C: Controller<T, W>> Widget<T> for ControllerHost<W, C> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.controller
            .event(&mut self.widget, ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.controller
            .lifecycle(&mut self.widget, ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.controller
            .update(&mut self.widget, ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.widget.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.widget.paint(ctx, data, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.widget.id()
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.widget.debug_state(data)],
            ..Default::default()
        }
    }

    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        self.widget.compute_max_intrinsic(axis, ctx, bc, data, env)
    }
}

impl<W, C> WidgetWrapper for ControllerHost<W, C> {
    widget_wrapper_body!(W, widget);
}
