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

//! A widget that aligns its child (for example, centering it).

use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point,
    Rect, Size, UpdateCtx, Widget, WidgetPod,
};

use crate::piet::UnitPoint;

/// A widget that aligns its child.
pub struct Align<T: Data> {
    align: UnitPoint,
    child: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Align<T> {
    /// Create widget with alignment.
    pub fn new(align: UnitPoint, child: impl Widget<T> + 'static) -> Align<T> {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
        }
    }

    /// Create centered widget.
    pub fn centered(child: impl Widget<T> + 'static) -> Align<T> {
        Align::new(UnitPoint::CENTER, child)
    }
}

impl<T: Data> Widget<T> for Align<T> {
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
        let size = self.child.layout(layout_ctx, &bc.loosen(), data, env);
        let my_size = bc.constrain(size);
        let extra_size = (my_size.to_vec2() - size.to_vec2()).to_size();
        let origin = self
            .align
            .resolve(Rect::from_origin_size(Point::ORIGIN, extra_size));
        self.child
            .set_layout_rect(Rect::from_origin_size(origin, size));
        my_size
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
