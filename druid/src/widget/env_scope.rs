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
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point, Rect, Size,
    UpdateCtx, Widget, WidgetPod,
};

/// A widget that accepts a closure to update the theme for its child.
pub struct EnvScope<T: Data, W: Widget<T>, F: Fn(&mut Env) + 'static> {
    f: F,
    child: WidgetPod<T, W>,
}

impl<T: Data, W: Widget<T>, F: Fn(&mut Env) + 'static> EnvScope<T, W, F> {
    /// Create a widget that themes its child.
    ///
    /// Accepts a closure that sets theme values.
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
    pub fn new(f: F, child: W) -> EnvScope<T, W, F> {
        EnvScope {
            f,
            child: WidgetPod::new(child),
        }
    }
}

impl<T: Data, W: Widget<T>, F: Fn(&mut Env) + 'static> Widget<T> for EnvScope<T, W, F> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _base_state: &BaseState, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.paint(paint_ctx, data, &new_env);
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

        let size = self.child.layout(layout_ctx, &bc, data, &new_env);
        self.child
            .set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));

        size
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.event(event, ctx, data, &new_env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env);

        self.child.update(ctx, data, &new_env);
    }
}
