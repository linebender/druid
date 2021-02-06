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
