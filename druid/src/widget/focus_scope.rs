// Copyright 2020 The druid Authors.
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

//! A focus scope widget.

use druid::widget::prelude::*;
use druid::{Data, FocusScopeNode, Widget, WidgetPod};

/// A Widget that serves as a scope for its descendants,
/// restricting focus traversal to the scoped controls.
pub struct FocusScope<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    focus_scope_node: FocusScopeNode,
}

impl<T: Data> FocusScope<T> {
    /// Create a new FocusScope widget with a child
    pub fn new(child: impl Widget<T> + 'static) -> Self {
        FocusScope {
            child: WidgetPod::new(child).boxed(),
            focus_scope_node: FocusScopeNode { widget_id: None },
        }
    }
}

impl<T: Data> Widget<T> for FocusScope<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        ctx.with_focus_scope(self.focus_scope_node, |ctx| {
            self.child.event(ctx, event, data, env);
        });
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.focus_scope_node.widget_id = Some(ctx.widget_id());
        }

        ctx.with_focus_scope(self.focus_scope_node, |ctx| {
            self.child.lifecycle(ctx, event, data, env);
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        ctx.with_focus_scope(self.focus_scope_node, |ctx| {
            self.child.update(ctx, data, env);
        });
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        ctx.with_focus_scope(self.focus_scope_node, |ctx| {
            let size = self.child.layout(ctx, &bc, data, env);
            let rect = size.to_rect();

            self.child.set_layout_rect(ctx, data, env, rect);
            size
        })
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_focus_scope(self.focus_scope_node, |ctx| {
            self.child.paint(ctx, data, env);
        });
    }
}
