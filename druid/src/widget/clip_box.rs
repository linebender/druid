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

use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
use crate::widget::prelude::*;
use crate::{Data, WidgetPod};

/// Represents the size and position of a rectangular "viewport" into a larger area.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Viewport {
    /// The size of the area that we have a viewport into.
    pub content_size: Size,
    /// The view rectangle.
    pub rect: Rect,
}

impl Viewport {
    /// Tries to find a position for the view rectangle that is contained in the content rectangle.
    ///
    /// If the supplied origin is good, returns it; if it isn't, we try to return the nearest
    /// origin that would make the view rectangle contained in the content rectangle. (This will
    /// fail if the content is smaller than the view, and we return `0.0` in each dimension where
    /// the content is smaller.)
    pub fn clamp_view_origin(&self, origin: Point) -> Point {
        let x = origin
            .x
            .min(self.content_size.width - self.rect.width())
            .max(0.0);
        let y = origin
            .y
            .min(self.content_size.height - self.rect.height())
            .max(0.0);
        Point::new(x, y)
    }

    /// Changes the viewport offset by `delta`, while trying to keep the view rectangle inside the
    /// content rectangle.
    ///
    /// Returns true if the offset actually changed. Even if `delta` is non-zero, the offset might
    /// not change. For example, if you try to move the viewport down but it is already at the
    /// bottom of the child widget, then the offset will not change and this function will return
    /// false.
    pub fn pan_by(&mut self, delta: Vec2) -> bool {
        self.pan_to(self.rect.origin() + delta)
    }

    /// Sets the viewport origin to `pos`, while trying to keep the view rectangle inside the
    /// content rectangle.
    ///
    /// Returns true if the position changed. Note that the valid values for the viewport origin
    /// are constrained by the size of the child, and so the origin might not get set to exactly
    /// `pos`.
    pub fn pan_to(&mut self, origin: Point) -> bool {
        let new_origin = self.clamp_view_origin(origin);
        if (new_origin - self.rect.origin()).hypot2() > 1e-12 {
            self.rect = self.rect.with_origin(new_origin);
            true
        } else {
            false
        }
    }

    /// Pan the smallest distance that makes the target [`Rect`] visible.
    ///
    /// If the target rect is larger than viewport size, we will prioritize
    /// the region of the target closest to its origin.
    pub fn pan_to_visible(&mut self, rect: Rect) -> bool {
        /// Given a position and the min and max edges of an axis,
        /// return a delta by which to adjust that axis such that the value
        /// falls between its edges.
        ///
        /// if the value already falls between the two edges, return 0.0.
        fn closest_on_axis(val: f64, min: f64, max: f64) -> f64 {
            assert!(min <= max);
            if val > min && val < max {
                0.0
            } else if val <= min {
                val - min
            } else {
                val - max
            }
        }

        // clamp the target region size to our own size.
        // this means we will show the portion of the target region that
        // includes the origin.
        let target_size = Size::new(
            rect.width().min(self.rect.width()),
            rect.height().min(self.rect.height()),
        );
        let rect = rect.with_size(target_size);

        let x0 = closest_on_axis(rect.min_x(), self.rect.min_x(), self.rect.max_x());
        let x1 = closest_on_axis(rect.max_x(), self.rect.min_x(), self.rect.max_x());
        let y0 = closest_on_axis(rect.min_y(), self.rect.min_y(), self.rect.max_y());
        let y1 = closest_on_axis(rect.max_y(), self.rect.min_y(), self.rect.max_y());

        let delta_x = if x0.abs() > x1.abs() { x0 } else { x1 };
        let delta_y = if y0.abs() > y1.abs() { y0 } else { y1 };
        let new_origin = self.rect.origin() + Vec2::new(delta_x, delta_y);
        self.pan_to(new_origin)
    }
}

/// A widget exposing a rectangular view into its child, which can be used as a building block for
/// widgets that scroll their child.
pub struct ClipBox<T, W> {
    child: WidgetPod<T, W>,
    port: Viewport,
    constrain_horizontal: bool,
    constrain_vertical: bool,
}

impl<T, W: Widget<T>> ClipBox<T, W> {
    /// Creates a new `ClipBox` wrapping `child`.
    pub fn new(child: W) -> Self {
        ClipBox {
            child: WidgetPod::new(child),
            port: Default::default(),
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

    /// Returns a the viewport describing this `ClipBox`'s position.
    pub fn viewport(&self) -> Viewport {
        self.port
    }

    /// Returns the size of the rectangular viewport into the child widget.
    /// To get the position of the viewport, see [`viewport_origin`].
    ///
    /// [`viewport_origin`]: struct.ClipBox.html#method.viewport_origin
    pub fn viewport_size(&self) -> Size {
        self.port.rect.size()
    }

    /// Returns the size of the child widget.
    pub fn content_size(&self) -> Size {
        self.port.content_size
    }

    /// Builder-style method for deciding whether to constrain the child horizontally. The default
    /// is `false`. See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Determine whether to constrain the child horizontally.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_horizontal(&mut self, constrain: bool) {
        self.constrain_horizontal = constrain;
    }

    /// Builder-style method for deciding whether to constrain the child vertically. The default
    /// is `false`.
    ///
    /// This setting affects how a `ClipBox` lays out its child.
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
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_vertical(&mut self, constrain: bool) {
        self.constrain_vertical = constrain;
    }

    /// Changes the viewport offset by `delta`.
    ///
    /// Returns true if the offset actually changed. Even if `delta` is non-zero, the offset might
    /// not change. For example, if you try to move the viewport down but it is already at the
    /// bottom of the child widget, then the offset will not change and this function will return
    /// false.
    pub fn pan_by(&mut self, delta: Vec2) -> bool {
        self.pan_to(self.viewport_origin() + delta)
    }

    /// Sets the viewport origin to `pos`.
    ///
    /// Returns true if the position changed. Note that the valid values for the viewport origin
    /// are constrained by the size of the child, and so the origin might not get set to exactly
    /// `pos`.
    pub fn pan_to(&mut self, origin: Point) -> bool {
        if self.port.pan_to(origin) {
            self.child
                .set_viewport_offset(self.viewport_origin().to_vec2());
            true
        } else {
            false
        }
    }

    /// Adjust the viewport to display as much of the target region as is possible.
    ///
    /// Returns `true` if the viewport changes.
    ///
    /// This will move the viewport the smallest distance that fully shows
    /// the target region. If the target region is larger than the viewport,
    /// we will display the portion that fits, prioritizing the portion closest
    /// to the origin.
    pub fn pan_to_visible(&mut self, region: Rect) -> bool {
        if self.port.pan_to_visible(region) {
            self.child
                .set_viewport_offset(self.viewport_origin().to_vec2());
            true
        } else {
            false
        }
    }

    /// Returns the origin of the viewport rectangle.
    pub fn viewport_origin(&self) -> Point {
        self.port.rect.origin()
    }

    /// Allows this `ClipBox`'s viewport rectangle to be modified. The provided callback function
    /// can modify its argument, and when it is done then this `ClipBox` will be modified to have
    /// the new viewport rectangle.
    pub fn with_port<F: FnOnce(&mut Viewport)>(&mut self, f: F) {
        f(&mut self.port);
        self.child
            .set_viewport_offset(self.viewport_origin().to_vec2());
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for ClipBox<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, ev: &Event, data: &mut T, env: &Env) {
        let viewport = ctx.size().to_rect();
        let force_event = self.child.is_hot() || self.child.is_active();
        if let Some(child_event) =
            ev.transform_scroll(self.viewport_origin().to_vec2(), viewport, force_event)
        {
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
        bc.debug_check("ClipBox");

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

        let content_size = self.child.layout(ctx, &child_bc, data, env);
        self.port.content_size = content_size;
        self.child.set_origin(ctx, data, env, Point::ORIGIN);

        self.port.rect = self.port.rect.with_size(bc.constrain(content_size));
        let new_offset = self.port.clamp_view_origin(self.viewport_origin());
        self.pan_to(new_offset);
        self.viewport_size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let viewport = ctx.size().to_rect();
        let offset = self.viewport_origin().to_vec2();
        ctx.with_save(|ctx| {
            ctx.clip(viewport);
            ctx.transform(Affine::translate(-offset));

            let mut visible = ctx.region().clone();
            visible += offset;
            ctx.with_child_ctx(visible, |ctx| self.child.paint_raw(ctx, data, env));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pan_to_visible() {
        let mut viewport = Viewport {
            content_size: Size::new(400., 400.),
            rect: Rect::from_origin_size((20., 20.), (20., 20.)),
        };

        assert!(!viewport.pan_to_visible(Rect::from_origin_size((22., 22.,), (5., 5.))));
        assert!(viewport.pan_to_visible(Rect::from_origin_size((10., 10.,), (5., 5.))));
        assert_eq!(viewport.rect.origin(), Point::new(10., 10.));
        assert_eq!(viewport.rect.size(), Size::new(20., 20.));
        assert!(!viewport.pan_to_visible(Rect::from_origin_size((10., 10.,), (50., 50.))));
        assert_eq!(viewport.rect.origin(), Point::new(10., 10.));

        assert!(viewport.pan_to_visible(Rect::from_origin_size((30., 10.,), (5., 5.))));
        assert_eq!(viewport.rect.origin(), Point::new(15., 10.));
        assert!(viewport.pan_to_visible(Rect::from_origin_size((5., 5.,), (5., 5.))));
        assert_eq!(viewport.rect.origin(), Point::new(5., 5.));
    }
}
