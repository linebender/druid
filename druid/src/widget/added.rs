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
//! [`Controller`]: struct.Controller.html
//! [`LifeCycle::WidgetAdded`]: enum.LifeCycle.html#variant.WidgetAdded

use crate::widget::Controller;
use crate::{Data, Env, LifeCycleCtx, Widget};
/// This [`Controller`] widget responds to [`LifeCycle::WidgetAdded`] event
/// with the provided closure. Pass this and a child widget to [`ControllerHost`]
/// to respond to the event when the child widget is added to the widget tree.
/// This is also available, for convenience, as an `on_added` method
/// via [`WidgetExt`].
///
/// [`Controller`]: struct.Controller.html
/// [`ControllerHost`]: struct.ControllerHost.html
/// [`WidgetExt`]: ../trait.WidgetExt.html
/// [`LifeCycle::WidgetAdded`]: enum.LifeCycle.html#variant.WidgetAdded
pub struct Added<T> {
    /// A closure that will be invoked when the child widget is added
    /// to the widget tree
    action: Box<dyn Fn(&mut LifeCycleCtx, &T, &Env)>,
}

impl<T: Data> Added<T> {
    /// Create a new [`Controller`] widget to respond to widget added to tree event.
    pub fn new(action: impl Fn(&mut LifeCycleCtx, &T, &Env) + 'static) -> Self {
        Self {
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for Added<T> {
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &crate::LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let crate::LifeCycle::WidgetAdded = event {
            (self.action)(ctx, data, env);
        }
        child.lifecycle(ctx, event, data, env)
    }
}
