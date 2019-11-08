// Copyright 2018 The xi-editor Authors.
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

//! A widget that themes its child.

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, UpdateCtx,
    Widget,
};

/// A widget that accepts a closure to update the theme for its child.
pub struct StyleChild<T: Data, F: Fn(&mut Env) + 'static> {
    f: F,
    child: Box<dyn Widget<T>>,
}

impl<T: Data, F: Fn(&mut Env) + 'static> StyleChild<T, F> {
    /// Create a widget that themes its child.
    ///
    /// Accepts a closure that sets theme values:
    /// ```
    /// # use druid::{theme, Widget};
    /// # use druid::piet::{Color};
    /// # use druid::widget::{Label, StyleChild};
    ///
    /// # fn build_width() -> impl Widget<String> {
    ///
    /// StyleChild::new(
    ///     |env| {
    ///         env.set(theme::LABEL_COLOR, Color::WHITE);
    ///     },
    ///     Label::new("White text!")
    /// )
    ///
    /// # }
    ///

    /// ```
    pub fn new(f: F, child: impl Widget<T> + 'static) -> StyleChild<T, F> {
        StyleChild {
            f,
            child: Box::new(child),
        }
    }
}

impl<T: Data, F: Fn(&mut Env) + 'static> Widget<T> for StyleChild<T, F> {
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
        bc.debug_check("StyleChild");

        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.layout(layout_ctx, &bc, data, &new_env)
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.event(event, ctx, data, &new_env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.update(ctx, old_data, data, &new_env);
    }
}
