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

//! A clickable [`Controller`] widget.
//!
//! [`Controller`]: struct.Controller.html

use crate::widget::Controller;
use crate::{Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, MouseButton, Widget};

/// A clickable [`Controller`] widget. Pass this and a child widget to a
/// [`ControllerHost`] to make the child interactive. More conveniently, this is
/// available as an `on_click` method via [`WidgetExt`]'.
///
/// This is an alternative to the standard [`Button`] widget, for when you want
/// to make an arbitrary widget clickable.
///
/// The child widget will also be updated on [`LifeCycle::HotChanged`] and
/// mouse down, which can be useful for painting based on `ctx.is_active()`
/// and `ctx.is_hot()`.
///
/// [`Controller`]: struct.Controller.html
/// [`ControllerHost`]: struct.ControllerHost.html
/// [`WidgetExt`]: ../trait.WidgetExt.html
/// [`Button`]: struct.Button.html
/// [`LifeCycle::HotChanged`]: ../enum.LifeCycle.html#variant.HotChanged
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
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == MouseButton::Left {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        (self.action)(ctx, data, env);
                    }
                    ctx.request_paint();
                }
            }
            _ => {}
        }

        child.event(ctx, event, data, env);
    }

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
