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

use crate::kurbo::{Size, Vec2};
use crate::widget::Viewport;
use crate::{
    scroll_component::*, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, UpdateCtx, Widget,
};

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
    viewport: Viewport<T, W>,
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
            viewport: Viewport::new(child),
            scroll_component: ScrollComponent::new(),
            direction: ScrollDirection::Bidirectional,
        }
    }

    /// Restrict scrolling to the vertical axis while locking child width.
    pub fn vertical(mut self) -> Self {
        self.direction = ScrollDirection::Vertical;
        self.viewport.set_constrain_vertical(false);
        self.viewport.set_constrain_horizontal(true);
        self
    }

    /// Restrict scrolling to the horizontal axis while locking child height.
    pub fn horizontal(mut self) -> Self {
        self.direction = ScrollDirection::Horizontal;
        self.viewport.set_constrain_vertical(true);
        self.viewport.set_constrain_horizontal(false);
        self
    }

    /// Returns a reference to the child widget.
    pub fn child(&self) -> &W {
        self.viewport.child()
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut(&mut self) -> &mut W {
        self.viewport.child_mut()
    }

    /// Returns the size of the child widget.
    pub fn child_size(&self) -> Size {
        self.viewport.content_size()
    }

    /// Returns the current scroll offset.
    pub fn offset(&self) -> Vec2 {
        self.viewport.viewport_offset()
    }

    /// Scroll `delta` units.
    ///
    /// Returns `true` if the scroll offset has changed.
    pub fn scroll_by(&mut self, delta: Vec2) -> bool {
        self.viewport.scroll_by(delta)
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Scroll<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.scroll_component
            .event(&mut self.viewport, ctx, event, env);
        if !ctx.is_handled() {
            self.viewport.event(ctx, event, data, env);
        }

        self.scroll_component
            .handle_scroll(&mut self.viewport, ctx, event, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.scroll_component.lifecycle(ctx, event, env);
        self.viewport.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.viewport.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Scroll");

        let child_size = self.viewport.layout(ctx, &bc, data, env);
        log_size_warnings(child_size);

        let self_size = bc.constrain(child_size);
        // The new size might have made the current scroll offset invalid. This makes it valid
        // again.
        let _ = self.scroll_by(Vec2::ZERO);
        self_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.viewport.paint(ctx, data, env);
        self.scroll_component.draw_bars(ctx, &self.viewport, env);
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
