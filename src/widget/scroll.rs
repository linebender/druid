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

//! A container that scrolls its contents.

use std::f64::INFINITY;

use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point,
    Rect, Size, UpdateCtx, Vec2, Widget, WidgetPod,
};

use crate::piet::RenderContext;

use crate::kurbo::Affine;

#[derive(Debug, Clone)]
enum ScrollDirection {
    Horizontal,
    Vertical,
    All,
}

impl ScrollDirection {
    /// Return the maximum size the container can be given
    /// its scroll direction and box constraints.
    /// In practice vertical scrolling will be width limited to
    /// box constraints and horizontal will be height limited.
    pub fn max_size(&self, bc: &BoxConstraints) -> Size {
        match self {
            ScrollDirection::Horizontal => Size::new(INFINITY, bc.max().height),
            ScrollDirection::Vertical => Size::new(bc.max().width, INFINITY),
            ScrollDirection::All => Size::new(INFINITY, INFINITY),
        }
    }
}

/// A container that scrolls its contents.
///
/// This container holds a single child, and uses the wheel to scroll it
/// when the child's bounds are larger than the viewport.
///
/// The child is laid out with completely unconstrained layout bounds.
pub struct Scroll<T: Data> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    child_size: Size,
    scroll_offset: Vec2,
    direction: ScrollDirection,
}

impl<T: Data> Scroll<T> {
    /// Create a new scroll container.
    ///
    /// This method will allow scrolling in all directions if child's bounds
    /// are larger than the viewport. Use [vertical](#method.vertical)
    /// and [horizontal](#method.horizontal) methods to limit scroll behavior.
    pub fn new(child: impl Widget<T> + 'static) -> Scroll<T> {
        Scroll {
            child: WidgetPod::new(child).boxed(),
            child_size: Default::default(),
            scroll_offset: Vec2::new(0.0, 0.0),
            direction: ScrollDirection::All,
        }
    }

    /// Update the scroll.
    ///
    /// Returns `true` if the scroll has been updated.
    fn scroll(&mut self, delta: Vec2, size: Size) -> bool {
        let mut offset = self.scroll_offset + delta;
        offset.x = offset.x.min(self.child_size.width - size.width).max(0.0);
        offset.y = offset.y.min(self.child_size.height - size.height).max(0.0);
        if (offset - self.scroll_offset).hypot2() > 1e-12 {
            self.scroll_offset = offset;
            true
        } else {
            false
        }
    }

    /// Limit scroll behavior to allow only vertical scrolling (Y-axis).
    /// The child is laid out with constrained width and infinite height.
    pub fn vertical(mut self) -> Self {
        self.direction = ScrollDirection::Vertical;
        self
    }

    /// Limit scroll behavior to allow only horizontal scrolling (X-axis).
    /// The child is laid out with constrained height and infinite width.
    pub fn horizontal(mut self) -> Self {
        self.direction = ScrollDirection::Horizontal;
        self
    }
}

impl<T: Data> Widget<T> for Scroll<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        if let Err(e) = paint_ctx.save() {
            eprintln!("error saving render context: {:?}", e);
            return;
        }
        let viewport = Rect::from_origin_size(Point::ORIGIN, base_state.size());
        paint_ctx.clip(viewport);
        paint_ctx.transform(Affine::translate(-self.scroll_offset));
        self.child.paint(paint_ctx, data, env);
        if let Err(e) = paint_ctx.restore() {
            eprintln!("error restoring render context: {:?}", e);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let child_bc = BoxConstraints::new(Size::ZERO, self.direction.max_size(bc));
        let size = self.child.layout(ctx, &child_bc, data, env);
        self.child_size = size;
        self.child
            .set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));
        let self_size = bc.constrain(Size::new(100.0, 100.0));
        let _ = self.scroll(Vec2::new(0.0, 0.0), self_size);
        self_size
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        let size = ctx.base_state.size();
        let viewport = Rect::from_origin_size(Point::ORIGIN, size);
        let child_event = event.transform_scroll(self.scroll_offset, viewport);
        let action = if let Some(child_event) = child_event {
            self.child.event(&child_event, ctx, data, env)
        } else {
            None
        };
        if !ctx.is_handled() {
            if let Event::Wheel(wheel) = event {
                if self.scroll(wheel.delta, size) {
                    ctx.invalidate();
                    ctx.set_handled();
                }
            }
        }
        action
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }
}
