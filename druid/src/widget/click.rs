// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A clickable [`Controller`] widget.

use crate::widget::Controller;
use crate::{Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, MouseButton, Widget};
use tracing::{instrument, trace};

/// A clickable [`Controller`] widget. Pass this and a child widget to a
/// [`ControllerHost`] to make the child interactive. More conveniently, this is
/// available as an [`on_click`] method via [`WidgetExt`].
///
/// This is an alternative to the standard [`Button`] widget, for when you want
/// to make an arbitrary widget clickable.
///
/// The child widget will also be updated on [`LifeCycle::HotChanged`] and
/// mouse down, which can be useful for painting based on `ctx.is_active()`
/// and `ctx.is_hot()`.
///
/// [`ControllerHost`]: super::ControllerHost
/// [`on_click`]: super::WidgetExt::on_click
/// [`WidgetExt`]: super::WidgetExt
/// [`Button`]: super::Button
pub struct Click<T> {
    /// A closure that will be invoked when the child widget is clicked.
    action: Box<dyn Fn(&mut EventCtx, &mut T, &Env)>,
}

impl<T: Data> Click<T> {
    /// Create a new clickable [`Controller`] widget.
    pub fn new(action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static) -> Self {
        Click {
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for Click<T> {
    #[instrument(
        name = "Click",
        level = "trace",
        skip(self, child, ctx, event, data, env)
    )]
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left && !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                    trace!("Widget {:?} pressed", ctx.widget_id());
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == MouseButton::Left {
                    ctx.set_active(false);
                    if ctx.is_hot() && !ctx.is_disabled() {
                        (self.action)(ctx, data, env);
                    }
                    ctx.request_paint();
                    trace!("Widget {:?} released", ctx.widget_id());
                }
            }
            _ => {}
        }

        child.event(ctx, event, data, env);
    }

    #[instrument(
        name = "Click",
        level = "trace",
        skip(self, child, ctx, event, data, env)
    )]
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let LifeCycle::HotChanged(_) | LifeCycle::FocusChanged(_) = event {
            ctx.request_paint();
        }

        child.lifecycle(ctx, event, data, env);
    }
}
