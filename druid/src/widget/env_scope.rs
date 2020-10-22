// Copyright 2019 The Druid Authors.
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

//! A widget that accepts a closure to update the environment for its child.

use crate::widget::prelude::*;
use crate::{Data, WidgetPod};

/// A widget that accepts a closure to update the environment for its child.
pub struct EnvScope<T, W> {
    pub(crate) f: Box<dyn Fn(&mut Env, &T)>,
    pub(crate) child: WidgetPod<T, W>,
}

impl<T, W: Widget<T>> EnvScope<T, W> {
    /// Create a widget that updates the environment for its descendants.
    ///
    /// Accepts a closure that sets Env values.
    ///
    /// This is available as [`WidgetExt::env_scope`] for convenience.
    ///
    /// # Examples
    /// ```
    /// # use druid::{theme, Widget};
    /// # use druid::piet::{Color};
    /// # use druid::widget::{Label, EnvScope};
    /// # fn build_widget() -> impl Widget<String> {
    /// EnvScope::new(
    ///     |env, data| {
    ///         env.set(theme::LABEL_COLOR, Color::WHITE);
    ///     },
    ///     Label::new("White text!")
    /// )
    ///
    /// # }
    /// ```
    ///
    /// [`WidgetExt::env_scope`]: ../trait.WidgetExt.html#method.env_scope
    pub fn new(f: impl Fn(&mut Env, &T) + 'static, child: W) -> EnvScope<T, W> {
        EnvScope {
            f: Box::new(f),
            child: WidgetPod::new(child),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for EnvScope<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, &data);

        self.child.event(ctx, event, data, &new_env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, &data);
        self.child.lifecycle(ctx, event, data, &new_env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, &data);

        self.child.update(ctx, data, &new_env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("EnvScope");

        let mut new_env = env.clone();
        (self.f)(&mut new_env, &data);

        let size = self.child.layout(ctx, &bc, data, &new_env);
        self.child.set_layout_rect(ctx, data, env, size.to_rect());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, &data);

        self.child.paint(ctx, data, &new_env);
    }
}
