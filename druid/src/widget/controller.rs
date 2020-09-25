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

//! A widget-controlling widget.

use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget, WidgetId,
};

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
/// ## A [`TextBox`] that takes focus on launch:
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
/// [`Widget`]: ../trait.Widget.html
/// [`TextBox`]: struct.TextBox.html
/// [`ControllerHost`]: struct.ControllerHost.html
/// [`WidgetExt::controller`]: ../trait.WidgetExt.html#tymethod.controller
pub trait Controller<T, W: Widget<T>> {
    /// Analogous to [`Widget::event`].
    ///
    /// [`Widget::event`]: ../trait.Widget.html#tymethod.event
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        child.event(ctx, event, data, env)
    }

    /// Analogous to [`Widget::lifecycle`].
    ///
    /// [`Widget::lifecycle`]: ../trait.Widget.html#tymethod.lifecycle
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
    ///
    /// [`Widget::update`]: ../trait.Widget.html#tymethod.update
    fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        child.update(ctx, old_data, data, env)
    }
}

/// A [`Widget`] that manages a child and a [`Controller`].
///
/// [`Widget`]: ../trait.Widget.html
/// [`Controller`]: trait.Controller.html
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
}
