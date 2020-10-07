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

use std::f64::INFINITY;

use crate::widget::prelude::*;
use crate::{scroll_component::*, Data, Vec2, WidgetPod};

#[derive(Debug, Clone)]
enum ScrollDirection {
    Bidirectional,
    Vertical,
    Horizontal,
}

/// A container that scrolls its contents.
///
/// This container holds a single child, and uses the wheel to scroll it
/// when the child's bounds are larger than the viewport.
///
/// The child is laid out with completely unconstrained layout bounds by
/// default. Restrict to a specific axis with [`vertical`] or [`horizontal`].
/// When restricted to scrolling on a specific axis the child's size is
/// locked on the opposite axis.
///
/// [`vertical`]: struct.Scroll.html#method.vertical
/// [`horizontal`]: struct.Scroll.html#method.horizontal
pub struct Scroll<T, W> {
    child: WidgetPod<T, W>,
    scroll_component: ScrollComponent,
    direction: ScrollDirection,
}

impl<T, W: Widget<T>> Scroll<T, W> {
    /// Create a new scroll container.
    ///
    /// This method will allow scrolling in all directions if child's bounds
    /// are larger than the viewport. Use [vertical](#method.vertical) and
    /// [horizontal](#method.horizontal) methods to limit scrolling to a specific axis.
    pub fn new(child: W) -> Scroll<T, W> {
        Scroll {
            child: WidgetPod::new(child),
            scroll_component: ScrollComponent::new(),
            direction: ScrollDirection::Bidirectional,
        }
    }

    /// Restrict scrolling to the vertical axis while locking child width.
    pub fn vertical(mut self) -> Self {
        self.direction = ScrollDirection::Vertical;
        self
    }

    /// Restrict scrolling to the horizontal axis while locking child height.
    pub fn horizontal(mut self) -> Self {
        self.direction = ScrollDirection::Horizontal;
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

    /// Scroll `delta` units.
    ///
    /// Returns `true` if the scroll offset has changed.
    pub fn scroll(&mut self, delta: Vec2, layout_size: Size) -> bool {
        let scrolled = self.scroll_component.scroll(delta, layout_size);
        self.child.set_viewport_offset(self.offset());
        scrolled
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Scroll<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.scroll_component.event(ctx, event, env);
        if !ctx.is_handled() {
            let viewport = ctx.size().to_rect();

            let force_event = self.child.is_hot() || self.child.is_active();
            let child_event =
                event.transform_scroll(self.scroll_component.scroll_offset, viewport, force_event);
            if let Some(child_event) = child_event {
                self.child.event(ctx, &child_event, data, env);
            };
        }

        self.scroll_component.handle_scroll(ctx, event, env);
        // In order to ensure that invalidation regions are correctly propagated up the tree,
        // we need to set the viewport offset on our child whenever we change our scroll offset.
        self.child.set_viewport_offset(self.offset());
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.scroll_component.lifecycle(ctx, event, env);
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Scroll");

        let max_bc = match self.direction {
            ScrollDirection::Bidirectional => Size::new(INFINITY, INFINITY),
            ScrollDirection::Vertical => Size::new(bc.max().width, INFINITY),
            ScrollDirection::Horizontal => Size::new(INFINITY, bc.max().height),
        };

        let child_bc = BoxConstraints::new(Size::ZERO, max_bc);
        let child_size = self.child.layout(ctx, &child_bc, data, env);
        log_size_warnings(child_size);
        let old_size = self.scroll_component.content_size;
        self.scroll_component.content_size = child_size;
        self.child
            .set_layout_rect(ctx, data, env, child_size.to_rect());

        let self_size = bc.constrain(child_size);
        let _ = self.scroll_component.scroll(Vec2::new(0.0, 0.0), self_size);
        self.child.set_viewport_offset(self.offset());

        if old_size != self.scroll_component.content_size {
            self.scroll_component
                .reset_scrollbar_fade(|d| ctx.request_timer(d), env);
        }

        self_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.scroll_component
            .paint_content(ctx, env, |visible, ctx| {
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
