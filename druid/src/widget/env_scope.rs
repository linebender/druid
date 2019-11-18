// Copyright 2019 The xi-editor Authors.
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

use std::marker::PhantomData;

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, UpdateCtx,
    Widget,
};

/// A widget that accepts a closure to update the environment for its child.
pub struct EnvScope<T: Data, W: Widget<T>> {
    f: Box<dyn Fn(&mut Env)>,
    child: W,
    phantom: PhantomData<T>,
}

impl<T: Data, W: Widget<T>> EnvScope<T, W> {
    /// Create a widget that updates the environment for its child.
    ///
    /// Accepts a closure that sets Env values.
    ///
    /// # Examples
    /// ```
    /// # use druid::{theme, Widget};
    /// # use druid::piet::{Color};
    /// # use druid::widget::{Label, EnvScope};
    ///
    /// # fn build_widget() -> impl Widget<String> {
    ///
    /// EnvScope::new(
    ///     |env| {
    ///         env.set(theme::LABEL_COLOR, Color::WHITE);
    ///     },
    ///     Label::new("White text!")
    /// )
    ///
    /// # }
    /// ```
    pub fn new(f: impl Fn(&mut Env) + 'static, child: W) -> EnvScope<T, W> {
        EnvScope {
            f: Box::new(f),
            child,
            phantom: Default::default(),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for EnvScope<T, W> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.paint(paint_ctx, base_state, data, &new_env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("EnvScope");

        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.layout(layout_ctx, &bc, data, &new_env)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.event(ctx, event, data, &new_env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.update(ctx, old_data, data, &new_env);
    }
}
