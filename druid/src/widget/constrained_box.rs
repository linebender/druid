// Copyright 2020 The xi-editor Authors.
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

//! A widget which modifies the BoxConstraints passed to its child

use crate::shell::kurbo::Size;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetId,
};

/// A widget with custom BoxConstraints.
///
/// The constrain function you provide should return valid BoxConstraints, and
/// isn't checked for "illegal" values.
pub struct ConstrainedBox<T> {
    inner: Box<dyn Widget<T>>,
    constrain: Box<dyn Fn(&BoxConstraints) -> BoxConstraints>,
}

impl<T> ConstrainedBox<T> {
    /// Create a new [`ConstrainedBox`] widget. The function provided can
    /// return arbitrary BoxConstraints, so please constrain responsibly.
    pub fn new(
        inner: impl Widget<T> + 'static,
        constrain: impl Fn(&BoxConstraints) -> BoxConstraints + 'static,
    ) -> Self {
        Self {
            inner: Box::new(inner),
            constrain: Box::new(constrain),
        }
    }
}

impl<T: Data> Widget<T> for ConstrainedBox<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("ConstrainedBox");

        let child_bc = (self.constrain)(bc);
        self.inner.layout(ctx, &child_bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
