// Copyright 2019 The Druid Authors.
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

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::{
    scroll_component::*, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, UpdateCtx, Widget, WidgetPod,
};

/// A container that scrolls its contents.
///
/// This container holds a single child, and uses the wheel to scroll it
/// when the child's bounds are larger than the viewport.
///
/// The child is laid out with completely unconstrained layout bounds.
pub struct Scroll<T, W> {
    child: WidgetPod<T, W>,
    scroll_component: ScrollComponent,
}

impl<T, W: Widget<T>> Scroll<T, W> {
    /// Create a new scroll container.
    ///
    /// This method will allow scrolling in all directions if child's bounds
    /// are larger than the viewport. Use [vertical](#method.vertical)
    /// and [horizontal](#method.horizontal) methods to limit scroll behavior.
    pub fn new(child: W) -> Scroll<T, W> {
        Scroll {
            child: WidgetPod::new(child),
            scroll_component: ScrollComponent::new(),
        }
    }

    /// Limit scroll behavior to allow only vertical scrolling (Y-axis).
    /// The child is laid out with constrained width and infinite height.
    pub fn vertical(mut self) -> Self {
        self.scroll_component.direction = ScrollDirection::Vertical;
        self
    }

    /// Limit scroll behavior to allow only horizontal scrolling (X-axis).
    /// The child is laid out with constrained height and infinite width.
    pub fn horizontal(mut self) -> Self {
        self.scroll_component.direction = ScrollDirection::Horizontal;
        self
    }

    /// Returns a reference to the child widget.
    pub fn child(&self) -> &W {
        self.child.widget()
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut(&mut self) -> &mut W {
        self.child.widget_mut()
    }

    /// Returns the size of the child widget.
    pub fn child_size(&self) -> Size {
        self.scroll_component.content_size
    }

    /// Returns the current scroll offset.
    pub fn offset(&self) -> Vec2 {
        self.scroll_component.scroll_offset
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Scroll<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if !self.scroll_component.filter_event(ctx, event, env) {
            let viewport = Rect::from_origin_size(Point::ORIGIN, ctx.size());

            let force_event = self.child.is_hot() || self.child.is_active();
            let child_event =
                event.transform_scroll(self.scroll_component.scroll_offset, viewport, force_event);
            if let Some(child_event) = child_event {
                self.child.event(ctx, &child_event, data, env);
            };
        }

        self.scroll_component.handle_scroll(ctx, event, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if !self.scroll_component.filter_lifecycle(ctx, event, env) {
            self.child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Scroll");

        let child_bc =
            BoxConstraints::new(Size::ZERO, self.scroll_component.direction.max_size(bc));
        let size = self.child.layout(ctx, &child_bc, data, env);
        log_size_warnings(size);

        self.scroll_component.content_size = size;
        self.child.set_layout_rect(ctx, data, env, size.to_rect());
        let self_size = bc.constrain(self.scroll_component.content_size);
        let _ = self.scroll_component.scroll(Vec2::new(0.0, 0.0), self_size);
        self_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.scroll_component
            .draw_content(ctx, env, |visible, ctx| {
                ctx.with_child_ctx(visible, |ctx| self.child.paint_raw(ctx, data, env));
            });
    }
}

fn log_size_warnings(size: Size) {
    if size.width.is_infinite() {
        log::warn!("Scroll widget's child has an infinite width.");
    }

    if size.height.is_infinite() {
        log::warn!("Scroll widget's child has an infinite height.");
    }
}
