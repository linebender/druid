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

//! A widget that switches dynamically between two child views.

use crate::widget::prelude::*;
use crate::{Data, WidgetPod};

/// A widget that switches between two possible child views.
pub struct Either<T> {
    closure: Box<dyn Fn(&T, &Env) -> bool>,
    true_branch: WidgetPod<T, Box<dyn Widget<T>>>,
    false_branch: WidgetPod<T, Box<dyn Widget<T>>>,
    current: bool,
}

impl<T> Either<T> {
    /// Create a new widget that switches between two views.
    ///
    /// The given closure is evaluated on data change. If its value is `true`, then
    /// the `true_branch` widget is shown, otherwise `false_branch`.
    pub fn new(
        closure: impl Fn(&T, &Env) -> bool + 'static,
        true_branch: impl Widget<T> + 'static,
        false_branch: impl Widget<T> + 'static,
    ) -> Either<T> {
        Either {
            closure: Box::new(closure),
            true_branch: WidgetPod::new(true_branch).boxed(),
            false_branch: WidgetPod::new(false_branch).boxed(),
            current: false,
        }
    }
}

impl<T: Data> Widget<T> for Either<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.current {
            self.true_branch.event(ctx, event, data, env)
        } else {
            self.false_branch.event(ctx, event, data, env)
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.current = (self.closure)(data, env);
        }
        self.true_branch.lifecycle(ctx, event, data, env);
        self.false_branch.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let current = (self.closure)(data, env);
        if current != self.current {
            self.current = current;
            ctx.request_layout();
        }
        if self.current {
            self.true_branch.update(ctx, data, env);
        } else {
            self.false_branch.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if self.current {
            let size = self.true_branch.layout(ctx, bc, data, env);
            self.true_branch
                .set_layout_rect(ctx, data, env, size.to_rect());
            ctx.set_paint_insets(self.true_branch.paint_insets());
            size
        } else {
            let size = self.false_branch.layout(ctx, bc, data, env);
            self.false_branch
                .set_layout_rect(ctx, data, env, size.to_rect());
            ctx.set_paint_insets(self.false_branch.paint_insets());
            size
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.current {
            self.true_branch.paint_raw(ctx, data, env);
        } else {
            self.false_branch.paint_raw(ctx, data, env);
        }
    }
}
