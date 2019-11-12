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

use std::iter::Map;
use std::iter::Zip;
use std::slice::Iter;
use std::slice::IterMut;

/// ListWidget contains common methods for List implementations.
/// `D` is List Data. `T` is List child Data.
trait ListWidget<'a, D: Data, T: Data + 'a> {
    fn children(&mut self) -> &mut Vec<WidgetPod<T, Box<dyn Widget<T>>>>;
    fn children_with_data(
        &mut self,
        data: &'a D,
    ) -> Zip<IterMut<'_, WidgetPod<T, Box<(dyn Widget<T>)>>>, Iter<'a, T>>;
    fn children_data(&mut self, data: &'a D) -> Iter<'a, T>;
    fn new_child(&self) -> WidgetPod<T, Box<dyn Widget<T>>>;

    fn list_paint(&mut self, paint_ctx: &mut PaintCtx, data: &'a D, env: &Env) {
        for (child, child_data) in self.children_with_data(data) {
            child.paint_with_offset(paint_ctx, child_data, env);
        }
    }

    fn list_layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &'a D,
        env: &Env,
    ) -> Size {
        let mut width = bc.min().width;
        let mut y = 0.0;
        for (child, child_data) in self.children_with_data(data) {
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

    fn list_update(&mut self, ctx: &mut UpdateCtx, data: &'a D, env: &Env) {
        for (child, child_data) in self.children_with_data(data) {
            child.update(ctx, child_data, env);
        }

        let data_len = self.children_data(data).count();
        let len = self.children().len();

        if len > data_len {
            self.children().truncate(data_len)
        } else if len < data_len {
            for child_data in self.children_data(data).skip(len) {
                let mut child = self.new_child();
                child.update(ctx, child_data, env);
                self.children().push(child);
            }
        }
    }
}

impl<'a, T: Data + 'a> ListWidget<'a, Arc<Vec<T>>, T> for List<T> {
    fn children(&mut self) -> &mut Vec<WidgetPod<T, Box<dyn Widget<T>>>> {
        &mut self.children
    }

    fn children_with_data(
        &mut self,
        data: &'a Arc<Vec<T>>,
    ) -> Zip<IterMut<'_, WidgetPod<T, Box<(dyn Widget<T>)>>>, Iter<'a, T>> {
        self.children().iter_mut().zip(data.iter())
    }

    fn children_data(&mut self, data: &'a Arc<Vec<T>>) -> Iter<'a, T> {
        data.iter()
    }

    fn new_child(&self) -> WidgetPod<T, Box<dyn Widget<T>>> {
        WidgetPod::new((self.closure)())
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
        self.list_paint(paint_ctx, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Arc<Vec<T>>,
        env: &Env,
    ) -> Size {
        self.list_layout(layout_ctx, bc, data, env)
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
        self.list_update(ctx, data, env);
    }
}

// Implementation for List with shared data

impl<'a, T1: Data + 'a, T: Data + 'a> ListWidget<'a, (T1, Arc<Vec<T>>), (T1, T)> for List<(T1, T)> {
    fn children(&mut self) -> &mut Vec<WidgetPod<(T1, T), Box<dyn Widget<(T1, T)>>>> {
        &mut self.children
    }

    // TODO: which return type to use to remove Vec construction?
    fn children_with_data(
        &mut self,
        data: &'a (T1, Arc<Vec<T>>),
    ) -> Zip<IterMut<'_, WidgetPod<(T1, T), Box<(dyn Widget<(T1, T)>)>>>, Iter<'a, (T1, T)>> {
        let list: Vec<(T1, T)> = data.1.iter().map(|c| (data.0.clone(), c.clone())).collect();
        self.children().iter_mut().zip(list.iter())
    }

    // TODO: which return type to use to remove Vec construction?
    fn children_data(&mut self, data: &'a (T1, Arc<Vec<T>>)) -> Iter<'a, (T1, T)> {
        let list: Vec<(T1, T)> = data.1.iter().map(|c| (data.0.clone(), c.clone())).collect();
        list.iter()
    }

    fn new_child(&self) -> WidgetPod<(T1, T), Box<dyn Widget<(T1, T)>>> {
        WidgetPod::new((self.closure)())
    }
}

impl<T1: Data, T: Data> Widget<(T1, Arc<Vec<T>>)> for List<(T1, T)> {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        _base_state: &BaseState,
        data: &(T1, Arc<Vec<T>>),
        env: &Env,
    ) {
        self.list_paint(paint_ctx, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &(T1, Arc<Vec<T>>),
        env: &Env,
    ) -> Size {
        self.list_layout(layout_ctx, bc, data, env)
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
        self.list_update(ctx, data, env);
    }
}
