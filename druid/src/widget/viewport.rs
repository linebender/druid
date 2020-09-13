// Copyright 2020 The Druid Authors.
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

use crate::kurbo::{Affine, Size, Vec2};
use crate::widget::prelude::*;
use crate::{Data, WidgetPod};

/// A widget exposing a rectangular view into its child, which can be used as a building block for
/// widgets that scroll their child.
pub struct Viewport<T, W> {
    child: WidgetPod<T, W>,
    offset: Vec2,
    viewport_size: Size,
    content_size: Size,
    constrain_horizontal: bool,
    constrain_vertical: bool,
}

impl<T, W: Widget<T>> Viewport<T, W> {
    /// Creates a new `Viewport` wrapping `child`.
    pub fn new(child: W) -> Self {
        Viewport {
            child: WidgetPod::new(child),
            offset: Vec2::ZERO,
            viewport_size: Size::ZERO,
            content_size: Size::ZERO,
            constrain_horizontal: false,
            constrain_vertical: false,
        }
    }

    /// Returns a reference to the child widget.
    pub fn child(&self) -> &W {
        self.child.widget()
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut(&mut self) -> &mut W {
        self.child.widget_mut()
    }

    /// Returns the size of the rectangular viewport into the child widget.
    /// To get the offset of the viewport, see [`viewport_offset`].
    ///
    /// [`viewport_offset`]: struct.Viewport.html#method.viewport_offset
    pub fn viewport_size(&self) -> Size {
        self.viewport_size
    }

    /// Returns the size of the child widget.
    pub fn content_size(&self) -> Size {
        self.content_size
    }

    /// Builder-style method for deciding whether to constrain the child horizontally. The default
    /// is `false`. See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.Viewport.html#constrain_vertical
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Determine whether to constrain the child horizontally.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.Viewport.html#constrain_vertical
    pub fn set_constrain_horizontal(&mut self, constrain: bool) {
        self.constrain_horizontal = constrain;
    }

    /// Builder-style method for deciding whether to constrain the child vertically. The default
    /// is `false`.
    ///
    /// This setting affects how a `Viewport` lays out its child.
    ///
    /// - When it is `false` (the default), the child does receive any upper bound on its height:
    ///   the idea is that the child can be as tall as it wants, and the viewport will somehow get
    ///   moved around to see all of it.
    /// - When it is `true`, the viewport's maximum height will be passed down as an upper bound on
    ///   the height of the child, and the viewport will set its own height to be the same as its
    ///   child's height.
    pub fn constrain_vertical(mut self, constrain: bool) -> Self {
        self.constrain_vertical = constrain;
        self
    }

    /// Determine whether to constrain the child vertically.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.Viewport.html#constrain_vertical
    pub fn set_constrain_vertical(&mut self, constrain: bool) {
        self.constrain_vertical = constrain;
    }

    /// Changes the viewport offset by `delta`.
    ///
    /// Returns true if the offset actually changed. Even if `delta` is non-zero, the offset might
    /// not change. For example, if you try to move the viewport down but it is already at the
    /// bottom of the child widget, then the offset will not change and this function will return
    /// false.
    pub fn scroll_by(&mut self, delta: Vec2) -> bool {
        self.scroll_to(self.offset + delta)
    }

    fn clamp_offset(&self, offset: Vec2) -> Vec2 {
        let x = offset
            .x
            .min(self.content_size.width - self.viewport_size.width)
            .max(0.0);
        let y = offset
            .y
            .min(self.content_size.height - self.viewport_size.height)
            .max(0.0);
        Vec2::new(x, y)
    }

    /// Sets the viewport offset to `offset`.
    ///
    /// Returns true if the offset changed. Note that the valid values for the viewport offset are
    /// constrained by the size of the child, and so the offset might not get set to exactly
    /// `offset`.
    pub fn scroll_to(&mut self, offset: Vec2) -> bool {
        let new_offset = self.clamp_offset(offset);
        if (new_offset - self.offset).hypot2() > 1e-12 {
            self.offset = new_offset;
            self.child.set_viewport_offset(new_offset);
            true
        } else {
            false
        }
    }

    /// Returns the offset of the viewport.
    pub fn viewport_offset(&self) -> Vec2 {
        self.offset
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Viewport<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, ev: &Event, data: &mut T, env: &Env) {
        let viewport = ctx.size().to_rect();
        let force_event = self.child.is_hot() || self.child.is_active();
        if let Some(child_event) = ev.transform_scroll(self.offset, viewport, force_event) {
            self.child.event(ctx, &child_event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, ev: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, ev, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Viewport");

        let max_child_width = if self.constrain_horizontal {
            bc.max().width
        } else {
            f64::INFINITY
        };
        let max_child_height = if self.constrain_vertical {
            bc.max().height
        } else {
            f64::INFINITY
        };
        let child_bc =
            BoxConstraints::new(Size::ZERO, Size::new(max_child_width, max_child_height));

        self.content_size = self.child.layout(ctx, &child_bc, data, env);
        self.child
            .set_layout_rect(ctx, data, env, self.content_size.to_rect());

        self.viewport_size = bc.constrain(self.content_size);
        let new_offset = self.clamp_offset(self.offset);
        self.scroll_to(new_offset);
        self.viewport_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let viewport = ctx.size().to_rect();
        ctx.with_save(|ctx| {
            ctx.clip(viewport);
            ctx.transform(Affine::translate(-self.offset));

            let mut visible = ctx.region().clone();
            visible += self.offset;
            ctx.with_child_ctx(visible, |ctx| self.child.paint_raw(ctx, data, env));
        });
    }
}
