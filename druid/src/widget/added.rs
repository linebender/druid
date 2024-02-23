// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A [`Controller`] widget that responds to [`LifeCycle::WidgetAdded`] event.
//!
//! [`Controller`]: crate::widget::Controller
//! [`LifeCycle::WidgetAdded`]: crate::LifeCycle::WidgetAdded

use crate::widget::Controller;
use crate::{Data, Env, LifeCycleCtx, Widget};
use tracing::{instrument, trace};

/// This [`Controller`] widget responds to [`LifeCycle::WidgetAdded`] event
/// with the provided closure. Pass this and a child widget to [`ControllerHost`]
/// to respond to the event when the child widget is added to the widget tree.
/// This is also available, for convenience, as an `on_added` method
/// via [`WidgetExt`].
///
/// [`Controller`]: crate::widget::Controller
/// [`ControllerHost`]: crate::widget::ControllerHost
/// [`WidgetExt`]: crate::widget::WidgetExt
/// [`LifeCycle::WidgetAdded`]: crate::LifeCycle::WidgetAdded
pub struct Added<T, W> {
    /// A closure that will be invoked when the child widget is added
    /// to the widget tree
    action: Box<dyn Fn(&mut W, &mut LifeCycleCtx, &T, &Env)>,
}

impl<T: Data, W: Widget<T>> Added<T, W> {
    /// Create a new [`Controller`] widget to respond to widget added to tree event.
    pub fn new(action: impl Fn(&mut W, &mut LifeCycleCtx, &T, &Env) + 'static) -> Self {
        Self {
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for Added<T, W> {
    #[instrument(
        name = "Added",
        level = "trace",
        skip(self, child, ctx, event, data, env)
    )]
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &crate::LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let crate::LifeCycle::WidgetAdded = event {
            trace!("Widget added");
            (self.action)(child, ctx, data, env);
        }
        child.lifecycle(ctx, event, data, env)
    }
}
