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

//! A widget that just adds padding during layout.

use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point,
    Rect, Size, UpdateCtx, Widget, WidgetPod,
};

pub struct Padding<T: Data> {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,

    child: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Padding<T> {
    /// Create widget with uniform padding.
    pub fn uniform(padding: f64, child: impl Widget<T> + 'static) -> Padding<T> {
        Padding {
            left: padding,
            right: padding,
            top: padding,
            bottom: padding,
            child: WidgetPod::new(child).boxed(),
        }
    }
}

impl<T: Data> Widget<T> for Padding<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _base_state: &BaseState, data: &T, env: &Env) {
        self.child.paint_with_offset(paint_ctx, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        let hpad = self.left + self.right;
        let vpad = self.top + self.bottom;
        let min = Size::new(bc.min.width - hpad, bc.min.height - vpad);
        let max = Size::new(bc.max.width - hpad, bc.max.height - vpad);
        let child_bc = BoxConstraints::new(min, max);
        let size = self.child.layout(layout_ctx, &child_bc, data, env);
        let origin = Point::new(self.left, self.top);
        self.child
            .set_layout_rect(Rect::from_origin_size(origin, size));
        Size::new(size.width + hpad, size.height + vpad)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        self.child.event(event, ctx, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }
}
