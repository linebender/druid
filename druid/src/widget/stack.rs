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

//! A widget that arranges its children on top of one another.

use crate::kurbo::Size;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetPod,
};

/// A container that lays out its children along the z-axis, first child at bottom, last child on top.
#[derive(Default)]
pub struct Stack<T> {
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
}

impl<T: Data> Stack<T> {
    /// Create a new stack layout.
    ///
    /// The child widgets are laid out on top of one another, from bottom to top.
    pub fn new() -> Self {
        Stack {
            children: Vec::new(),
        }
    }

    /// Builder-style variant of `add_child`.
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(mut self, child: impl Widget<T> + 'static) -> Self {
        self.add_child(child);
        self
    }

    /// Add a child widget.
    ///
    /// See also `with_child`.
    pub fn add_child(&mut self, child: impl Widget<T> + 'static) {
        self.children.push(WidgetPod::new(child).boxed());
    }
}

impl<T: Data> Widget<T> for Stack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(e) | Event::MouseUp(e) | Event::MouseMove(e) => {
                if let Some(active_child) = self
                    .children
                    .iter_mut()
                    .rev()
                    .find(|child| child.layout_rect().contains(e.pos))
                {
                    active_child.event(ctx, event, data, env);
                }
            }
            _ => {
                for child in &mut self.children.iter_mut().rev() {
                    child.event(ctx, event, data, env);
                    if ctx.is_handled() {
                        break;
                    }
                }
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for child in &mut self.children {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        for child in &mut self.children {
            child.update(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Stack");
        let loosened_bc = bc.loosen();
        let mut max_width = 0.0f64;
        let mut max_height = 0.0f64;
        for child in &mut self.children {
            let child_size: Size = child.layout(layout_ctx, &loosened_bc, data, env);
            max_width = max_width.max(child_size.width);
            max_height = max_height.max(child_size.height);
            // Stash size.
            let rect = child_size.to_rect();
            child.set_layout_rect(rect);
        }
        Size::new(max_width, max_height)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        for child in &mut self.children {
            child.paint_with_offset(paint_ctx, data, env);
        }
    }
}
