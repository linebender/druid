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

//! A widget that can dynamically switch between one of many views.

use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, Size, UpdateCtx, Widget, WidgetPod,
};

/// A widget that can switch dynamically between one of many views depending
/// application state.
pub struct ViewSwitcher<T: Data, U: PartialEq + Clone> {
    child_selector: Box<dyn Fn(&T, &Env) -> U>,
    child_builder: Box<dyn Fn(U, &Env) -> Box<dyn Widget<T>>>,
    active_child: Option<WidgetPod<T, Box<dyn Widget<T>>>>,
    active_child_id: Option<U>,
}

impl<T: Data, U: PartialEq + Clone> ViewSwitcher<T, U> {
    /// Create a new view switcher.
    ///
    /// The `child_selector` closure is called every time the application data changes.
    /// If the value it returns is the same as the one it returned during the previous
    /// data change, nothing happens. If it returns a different value, then the
    /// `child_builder` closure is called with the new value.
    ///
    /// The `child_builder` closure is expected to create a new child widget depending
    /// on the selector to it.
    pub fn new(
        child_selector: impl Fn(&T, &Env) -> U + 'static,
        child_builder: impl Fn(U, &Env) -> Box<dyn Widget<T>> + 'static,
    ) -> Self {
        Self {
            child_selector: Box::new(child_selector),
            child_builder: Box::new(child_builder),
            active_child: None,
            active_child_id: None,
        }
    }
}

impl<T: Data, U: PartialEq + Clone> Widget<T> for ViewSwitcher<T, U> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(ref mut child) = self.active_child {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            let child_id = (self.child_selector)(data, env);
            self.active_child_id = Some(child_id.clone());
            self.active_child = Some(WidgetPod::new((self.child_builder)(child_id, env)));
        }
        if let Some(ref mut child) = self.active_child {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        let child_id = (self.child_selector)(data, env);
        match &self.active_child_id {
            Some(current_child_id) => {
                if &child_id != current_child_id {
                    self.active_child =
                        Some(WidgetPod::new((self.child_builder)(child_id.clone(), env)));
                    self.active_child_id = Some(child_id);
                    ctx.children_changed();
                }
            }
            None => {}
        }

        if !old_data.same(data) {
            ctx.invalidate();
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        match self.active_child {
            Some(ref mut child) => {
                let size = child.layout(layout_ctx, bc, data, env);
                child.set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));
                size
            }
            None => bc.max(),
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut child) = self.active_child {
            child.paint(ctx, data, env);
        }
    }
}
