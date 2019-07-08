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

//! A widget that listens for events and invokes a closure.

use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point,
    Rect, Size, UpdateCtx, Widget,
};

pub struct ActionWrapper<T: Data, F: FnMut(&mut T, &Env)> {
    child: Box<dyn Widget<T>>,
    closure: F,
}

impl<T: Data, F: FnMut(&mut T, &Env)> ActionWrapper<T, F> {
    /// Create widget with uniform padding.
    pub fn new(child: impl Widget<T> + 'static, closure: F) -> ActionWrapper<T, F> {
        ActionWrapper {
            child: Box::new(child),
            closure,
        }
    }
}

impl<T: Data, F: FnMut(&mut T, &Env)> Widget<T> for ActionWrapper<T, F> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        self.child.paint(paint_ctx, base_state, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        self.child.layout(layout_ctx, bc, data, env)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        // Consume all actions; other possiblities include matching on details
        // of the action, or combining this with the button.
        if let Some(_action) = self.child.event(event, ctx, data, env) {
            (self.closure)(data, env);
        }
        None
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }
}
