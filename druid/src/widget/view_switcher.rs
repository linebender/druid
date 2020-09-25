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

//! A widget that can dynamically switch between one of many views.

use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, Size, UpdateCtx, Widget, WidgetPod,
};

/// A widget that can switch dynamically between one of many views depending
/// on application state.

type ChildPicker<T, U> = dyn Fn(&T, &Env) -> U;
type ChildBuilder<T, U> = dyn Fn(&U, &T, &Env) -> Box<dyn Widget<T>>;

/// A widget that dynamically switches between two children.
pub struct ViewSwitcher<T, U> {
    child_picker: Box<ChildPicker<T, U>>,
    child_builder: Box<ChildBuilder<T, U>>,
    active_child: Option<WidgetPod<T, Box<dyn Widget<T>>>>,
    active_child_id: Option<U>,
}

impl<T: Data, U: Data> ViewSwitcher<T, U> {
    /// Create a new view switcher.
    ///
    /// The `child_picker` closure is called every time the application data changes.
    /// If the value it returns is the same as the one it returned during the previous
    /// data change, nothing happens. If it returns a different value, then the
    /// `child_builder` closure is called with the new value.
    ///
    /// The `child_builder` closure creates a new child widget based on
    /// the value passed to it.
    pub fn new(
        child_picker: impl Fn(&T, &Env) -> U + 'static,
        child_builder: impl Fn(&U, &T, &Env) -> Box<dyn Widget<T>> + 'static,
    ) -> Self {
        Self {
            child_picker: Box::new(child_picker),
            child_builder: Box::new(child_builder),
            active_child: None,
            active_child_id: None,
        }
    }
}

impl<T: Data, U: Data> Widget<T> for ViewSwitcher<T, U> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(child) = self.active_child.as_mut() {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            let child_id = (self.child_picker)(data, env);
            self.active_child = Some(WidgetPod::new((self.child_builder)(&child_id, data, env)));
            self.active_child_id = Some(child_id);
        }
        if let Some(child) = self.active_child.as_mut() {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let child_id = (self.child_picker)(data, env);
        // Safe to unwrap because self.active_child_id should not be empty
        if !child_id.same(self.active_child_id.as_ref().unwrap()) {
            self.active_child = Some(WidgetPod::new((self.child_builder)(&child_id, data, env)));
            self.active_child_id = Some(child_id);
            ctx.children_changed();
        // Because the new child has not yet been initialized, we have to skip the update after switching.
        } else if let Some(child) = self.active_child.as_mut() {
            child.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        match self.active_child {
            Some(ref mut child) => {
                let size = child.layout(ctx, bc, data, env);
                child.set_layout_rect(ctx, data, env, Rect::from_origin_size(Point::ORIGIN, size));
                size
            }
            None => bc.max(),
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut child) = self.active_child {
            child.paint_raw(ctx, data, env);
        }
    }
}
