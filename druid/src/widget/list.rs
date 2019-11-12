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

//! Simple list view widget.

use std::sync::Arc;

use crate::kurbo::{Point, Rect, Size};

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
    WidgetPod,
};

pub struct List<T: Data> {
    closure: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
}

impl<T: Data> List<T> {
    pub fn new(closure: impl Fn() -> Box<dyn Widget<T>> + 'static) -> Self {
        List {
            closure: Box::new(closure),
            children: Vec::new(),
        }
    }
}

impl<T: Data> Widget<Arc<Vec<T>>> for List<T> {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        data: &Arc<Vec<T>>,
        env: &Env,
    ) {
        for (child, child_data) in self.children.iter_mut().zip(data.iter()) {
            child.paint_with_offset(paint_ctx, child_data, env);
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Arc<Vec<T>>,
        env: &Env,
    ) -> Size {
        let mut width = bc.min().width;
        let mut y = 0.0;
        for (child, child_data) in self.children.iter_mut().zip(data.iter()) {
            let child_bc = BoxConstraints::new(
                Size::new(bc.min().width, 0.0),
                Size::new(bc.max().width, std::f64::INFINITY),
            );
            let child_size = child.layout(layout_ctx, &child_bc, child_data, env);
            let rect = Rect::from_origin_size(Point::new(0.0, y), child_size);
            child.set_layout_rect(rect);
            width = width.max(child_size.width);
            y += child_size.height;
        }
        bc.constrain(Size::new(width, y))
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut Arc<Vec<T>>, env: &Env) {
        let mut new_data = Vec::with_capacity(data.len());
        let mut any_changed = false;
        for (child, child_data) in self.children.iter_mut().zip(data.iter()) {
            let mut d = child_data.to_owned();
            child.event(event, ctx, &mut d, env);
            if !any_changed && !child_data.same(&d) {
                any_changed = true;
            }
            new_data.push(d);
        }
        if any_changed {
            *data = Arc::new(new_data);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: Option<&Arc<Vec<T>>>,
        data: &Arc<Vec<T>>,
        env: &Env,
    ) {
        for (child, child_data) in self.children.iter_mut().zip(data.iter()) {
            child.update(ctx, child_data, env);
        }
        let len = self.children.len();
        if len > data.len() {
            self.children.truncate(data.len())
        } else if len < data.len() {
            for child_data in &data[len..] {
                let mut child = WidgetPod::new((self.closure)());
                child.update(ctx, child_data, env);
                self.children.push(child);
            }
        }
    }
}

// This is cut'n'paste for now to support both plain lists and lists paired with
// shared data, but it should migrate to a list-iteration trait.

impl<T1: Data, T: Data> Widget<(T1, Arc<Vec<T>>)> for List<(T1, T)> {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        data: &(T1, Arc<Vec<T>>),
        env: &Env,
    ) {
        for (child, child_data) in self.children.iter_mut().zip(data.1.iter()) {
            let d = (data.0.clone(), child_data.to_owned());
            child.paint_with_offset(paint_ctx, &d, env);
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &(T1, Arc<Vec<T>>),
        env: &Env,
    ) -> Size {
        let mut width = bc.min().width;
        let mut y = 0.0;
        for (child, child_data) in self.children.iter_mut().zip(data.1.iter()) {
            let d = (data.0.clone(), child_data.to_owned());
            let child_bc = BoxConstraints::new(
                Size::new(bc.min().width, 0.0),
                Size::new(bc.max().width, std::f64::INFINITY),
            );
            let child_size = child.layout(layout_ctx, &child_bc, &d, env);
            let rect = Rect::from_origin_size(Point::new(0.0, y), child_size);
            child.set_layout_rect(rect);
            width = width.max(child_size.width);
            y += child_size.height;
        }
        bc.constrain(Size::new(width, y))
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut (T1, Arc<Vec<T>>),
        env: &Env,
    ) {
        let mut new_shared = data.0.to_owned();
        let mut new_data = Vec::with_capacity(data.1.len());
        let mut any_shared_changed = false;
        let mut any_el_changed = false;
        for (child, child_data) in self.children.iter_mut().zip(data.1.iter()) {
            let mut d = (new_shared.clone(), child_data.to_owned());
            child.event(event, ctx, &mut d, env);
            if !any_shared_changed && !new_shared.same(&d.0) {
                any_shared_changed = true;
            }
            if any_shared_changed {
                new_shared = d.0;
            }
            if !any_el_changed && !child_data.same(&d.1) {
                any_el_changed = true;
            }
            new_data.push(d.1);
        }
        // It's not clear we need to track this; it's possible it would
        // be slightly more efficient to just update data.0 in place.
        if any_shared_changed {
            data.0 = new_shared;
        }
        if any_el_changed {
            data.1 = Arc::new(new_data);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: Option<&(T1, Arc<Vec<T>>)>,
        data: &(T1, Arc<Vec<T>>),
        env: &Env,
    ) {
        for (child, child_data) in self.children.iter_mut().zip(data.1.iter()) {
            let d = (data.0.clone(), child_data.to_owned());
            child.update(ctx, &d, env);
        }
        let len = self.children.len();
        if len > data.1.len() {
            self.children.truncate(data.1.len())
        } else if len < data.1.len() {
            for child_data in &data.1[len..] {
                let mut child = WidgetPod::new((self.closure)());
                let d = (data.0.clone(), child_data.to_owned());
                child.update(ctx, &d, env);
                self.children.push(child);
            }
        }
    }
}
