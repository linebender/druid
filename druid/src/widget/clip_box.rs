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

use crate::commands::SCROLL_TO_VIEW;
use crate::debug_state::DebugState;
use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
use crate::widget::prelude::*;
use crate::widget::Axis;
use crate::{Data, WidgetPod};
use tracing::{info, instrument, trace, warn};

/// Represents the size and position of a rectangular "viewport" into a larger area.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Viewport {
    /// The size of the area that we have a viewport into.
    pub content_size: Size,
    /// The origin of the view rectangle, relative to the content.
    pub view_origin: Point,
    /// The size of the view rectangle.
    pub view_size: Size,
}

impl Viewport {
    /// The view rectangle.
    pub fn view_rect(&self) -> Rect {
        Rect::from_origin_size(self.view_origin, self.view_size)
    }

    /// Tries to find a position for the view rectangle that is contained in the content rectangle.
    ///
    /// If the supplied origin is good, returns it; if it isn't, we try to return the nearest
    /// origin that would make the view rectangle contained in the content rectangle. (This will
    /// fail if the content is smaller than the view, and we return `0.0` in each dimension where
    /// the content is smaller.)
    pub fn clamp_view_origin(&self, origin: Point) -> Point {
        let x = origin
            .x
            .min(self.content_size.width - self.view_size.width)
            .max(0.0);
        let y = origin
            .y
            .min(self.content_size.height - self.view_size.height)
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
        self.pan_to(self.view_origin + delta)
    }

    /// Sets the viewport origin to `pos`, while trying to keep the view rectangle inside the
    /// content rectangle.
    ///
    /// Returns true if the position changed. Note that the valid values for the viewport origin
    /// are constrained by the size of the child, and so the origin might not get set to exactly
    /// `pos`.
    pub fn pan_to(&mut self, origin: Point) -> bool {
        let new_origin = self.clamp_view_origin(origin);
        if (new_origin - self.view_origin).hypot2() > 1e-12 {
            self.view_origin = new_origin;
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
            rect.width().min(self.view_size.width),
            rect.height().min(self.view_size.height),
        );
        let rect = rect.with_size(target_size);

        let my_rect = self.view_rect();
        let x0 = closest_on_axis(rect.min_x(), my_rect.min_x(), my_rect.max_x());
        let x1 = closest_on_axis(rect.max_x(), my_rect.min_x(), my_rect.max_x());
        let y0 = closest_on_axis(rect.min_y(), my_rect.min_y(), my_rect.max_y());
        let y1 = closest_on_axis(rect.max_y(), my_rect.min_y(), my_rect.max_y());

        let delta_x = if x0.abs() > x1.abs() { x0 } else { x1 };
        let delta_y = if y0.abs() > y1.abs() { y0 } else { y1 };
        let new_origin = self.view_origin + Vec2::new(delta_x, delta_y);
        self.pan_to(new_origin)
    }

    /// The default handling of the [`SCROLL_TO_VIEW`] notification for a scrolling container.
    ///
    /// The [`SCROLL_TO_VIEW`] notification is send when [`scroll_to_view`] or [`scroll_area_to_view`]
    /// are called.
    ///
    /// [`SCROLL_TO_VIEW`]: crate::commands::SCROLL_TO_VIEW
    /// [`scroll_to_view`]: crate::EventCtx::scroll_to_view()
    /// [`scroll_area_to_view`]: crate::EventCtx::scroll_area_to_view()
    pub fn default_scroll_to_view_handling(
        &mut self,
        ctx: &mut EventCtx,
        global_highlight_rect: Rect,
    ) -> bool {
        let mut viewport_changed = false;
        let global_content_offset = ctx.window_origin().to_vec2() - self.view_origin.to_vec2();
        let content_highlight_rect = global_highlight_rect - global_content_offset;

        if self
            .content_size
            .to_rect()
            .intersect(content_highlight_rect)
            != content_highlight_rect
        {
            warn!("tried to bring area outside of the content to view!");
        }

        if self.pan_to_visible(content_highlight_rect) {
            ctx.request_paint();
            viewport_changed = true;
        }
        // This is a new value since view_origin has changed in the meantime
        let global_content_offset = ctx.window_origin().to_vec2() - self.view_origin.to_vec2();
        ctx.submit_notification_without_warning(
            SCROLL_TO_VIEW.with(content_highlight_rect + global_content_offset),
        );
        viewport_changed
    }

    /// This method handles SCROLL_TO_VIEW by clipping the view_rect to the content rect.
    ///
    /// The [`SCROLL_TO_VIEW`] notification is send when [`scroll_to_view`] or [`scroll_area_to_view`]
    /// are called.
    ///
    /// [`SCROLL_TO_VIEW`]: crate::commands::SCROLL_TO_VIEW
    /// [`scroll_to_view`]: crate::EventCtx::scroll_to_view()
    /// [`scroll_area_to_view`]: crate::EventCtx::scroll_area_to_view()
    pub fn fixed_scroll_to_view_handling(
        &self,
        ctx: &mut EventCtx,
        global_highlight_rect: Rect,
        source: WidgetId,
    ) {
        let global_viewport_rect = self.view_rect() + ctx.window_origin().to_vec2();
        let clipped_highlight_rect = global_highlight_rect.intersect(global_viewport_rect);

        if clipped_highlight_rect.area() > 0.0 {
            ctx.submit_notification_without_warning(SCROLL_TO_VIEW.with(clipped_highlight_rect));
        } else {
            info!("Hidden Widget({}) in unmanaged clip requested SCROLL_TO_VIEW. The request is ignored.", source.to_raw());
        }
    }
}

/// A widget exposing a rectangular view into its child, which can be used as a building block for
/// widgets that scroll their child.
pub struct ClipBox<T, W> {
    child: WidgetPod<T, W>,
    port: Viewport,
    constrain_horizontal: bool,
    constrain_vertical: bool,
    must_fill: bool,
    old_bc: BoxConstraints,

    //This ClipBox is wrapped by a widget which manages the viewport_offset
    managed: bool,
}

impl<T, W> ClipBox<T, W> {
    /// Builder-style method for deciding whether to constrain the child vertically.
    ///
    /// The default is `false`.
    ///
    /// This setting affects how a `ClipBox` lays out its child.
    ///
    /// - When it is `false` (the default), the child does not receive any upper
    ///   bound on its height: the idea is that the child can be as tall as it
    ///   wants, and the viewport will somehow get moved around to see all of it.
    /// - When it is `true`, the viewport's maximum height will be passed down
    ///   as an upper bound on the height of the child, and the viewport will set
    ///   its own height to be the same as its child's height.
    pub fn constrain_vertical(mut self, constrain: bool) -> Self {
        self.constrain_vertical = constrain;
        self
    }

    /// Builder-style method for deciding whether to constrain the child horizontally.
    ///
    /// The default is `false`. See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn constrain_horizontal(mut self, constrain: bool) -> Self {
        self.constrain_horizontal = constrain;
        self
    }

    /// Builder-style method to set whether the child must fill the view.
    ///
    /// If `false` (the default) there is no minimum constraint on the child's
    /// size. If `true`, the child is passed the same minimum constraints as
    /// the `ClipBox`.
    pub fn content_must_fill(mut self, must_fill: bool) -> Self {
        self.must_fill = must_fill;
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

    /// Returns a the viewport describing this `ClipBox`'s position.
    pub fn viewport(&self) -> Viewport {
        self.port
    }

    /// Returns the origin of the viewport rectangle.
    pub fn viewport_origin(&self) -> Point {
        self.port.view_origin
    }

    /// Returns the size of the rectangular viewport into the child widget.
    /// To get the position of the viewport, see [`viewport_origin`].
    ///
    /// [`viewport_origin`]: struct.ClipBox.html#method.viewport_origin
    pub fn viewport_size(&self) -> Size {
        self.port.view_size
    }

    /// Returns the size of the child widget.
    pub fn content_size(&self) -> Size {
        self.port.content_size
    }

    /// Set whether to constrain the child horizontally.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_horizontal(&mut self, constrain: bool) {
        self.constrain_horizontal = constrain;
    }

    /// Set whether to constrain the child vertically.
    ///
    /// See [`constrain_vertical`] for more details.
    ///
    /// [`constrain_vertical`]: struct.ClipBox.html#constrain_vertical
    pub fn set_constrain_vertical(&mut self, constrain: bool) {
        self.constrain_vertical = constrain;
    }

    /// Set whether the child's size must be greater than or equal the size of
    /// the `ClipBox`.
    ///
    /// See [`content_must_fill`] for more details.
    ///
    /// [`content_must_fill`]: ClipBox::content_must_fill
    pub fn set_content_must_fill(&mut self, must_fill: bool) {
        self.must_fill = must_fill;
    }
}

impl<T, W: Widget<T>> ClipBox<T, W> {
    /// Creates a new `ClipBox` wrapping `child`.
    ///
    /// This method should only be used when creating your own widget, which uses ClipBox
    /// internally.
    ///
    /// `ClipBox` will forward [`SCROLL_TO_VIEW`] notifications to its parent unchanged.
    /// In this case the parent has to handle said notification itself. By default the ClipBox will
    /// filter out [`SCROLL_TO_VIEW`] notifications which refer to areas not visible.
    ///
    /// [`SCROLL_TO_VIEW`]: crate::commands::SCROLL_TO_VIEW
    pub fn managed(child: W) -> Self {
        ClipBox {
            child: WidgetPod::new(child),
            port: Default::default(),
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
            old_bc: BoxConstraints::tight(Size::ZERO),
            managed: true,
        }
    }

    /// Creates a new unmanaged `ClipBox` wrapping `child`.
    ///
    /// This method should be used when you are using ClipBox in the widget-hierachie directly.
    pub fn unmanaged(child: W) -> Self {
        ClipBox {
            child: WidgetPod::new(child),
            port: Default::default(),
            constrain_horizontal: false,
            constrain_vertical: false,
            must_fill: false,
            old_bc: BoxConstraints::tight(Size::ZERO),
            managed: false,
        }
    }

    /// Changes the viewport offset by `delta`.
    ///
    /// Returns true if the offset actually changed. Even if `delta` is non-zero, the offset might
    /// not change. For example, if you try to move the viewport down but it is already at the
    /// bottom of the child widget, then the offset will not change and this function will return
    /// false.
    pub fn pan_by(&mut self, ctx: &mut EventCtx, delta: Vec2) -> bool {
        self.pan_to(ctx: &mut EventCtx, self.viewport_origin() + delta)
    }

    /// Changes the viewport offset on the specified axis to 'position'.
    ///
    /// The other axis will remain unchanged.
    pub fn pan_to_on_axis(&mut self, ctx: &mut EventCtx, axis: Axis, position: f64) -> bool {
        let new_origin = axis.pack(position, axis.minor_pos(self.viewport_origin()));
        self.with_port(ctx, |ctx, port| {port.pan_to(new_origin.into());})
    }

    /// Sets the viewport origin to `pos`.
    ///
    /// Returns true if the position changed. Note that the valid values for the viewport origin
    /// are constrained by the size of the child, and so the origin might not get set to exactly
    /// `pos`.
    pub fn pan_to(&mut self, ctx: &mut EventCtx, origin: Point) -> bool {
        self.with_port(ctx, |ctx, port|{port.pan_to(origin);})
    }

    /// Adjust the viewport to display as much of the target region as is possible.
    ///
    /// Returns `true` if the viewport changes.
    ///
    /// This will move the viewport the smallest distance that fully shows
    /// the target region. If the target region is larger than the viewport,
    /// we will display the portion that fits, prioritizing the portion closest
    /// to the origin.
    pub fn pan_to_visible(&mut self, ctx: &mut EventCtx, region: Rect) -> bool {
        self.with_port(ctx, |ctx, port|{port.pan_to_visible(region);})
    }

    /// Modify the `ClipBox`'s viewport rectangle with a closure.
    ///
    /// The provided callback function can modify its argument, and when it is
    /// done then this `ClipBox` will be modified to have the new viewport rectangle.
    pub fn with_port<F: FnOnce(&mut EventCtx, &mut Viewport)>(&mut self, ctx: &mut EventCtx, f: F) -> bool {
        f(ctx, &mut self.port);
        let new_content_origin = -self.port.view_origin;

        if new_content_origin != self.child.layout_rect().origin() {
                self.child
                    .set_origin_dyn(ctx, new_content_origin);
            true
        } else {
            false
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for ClipBox<T, W> {
    #[instrument(name = "ClipBox", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Notification(notification) = event {
            if let Some(global_highlight_rect) = notification.get(SCROLL_TO_VIEW) {
                if !self.managed {
                    // If the parent widget does not handle SCROLL_TO_VIEW notifications, we
                    // prevent unexpected behaviour, by clipping SCROLL_TO_VIEW notifications
                    // to this ClipBox's viewport.
                    ctx.set_handled();
                    self.with_port(ctx, |ctx, port| {
                        port.fixed_scroll_to_view_handling(
                            ctx,
                            *global_highlight_rect,
                            notification.source(),
                        );
                    });
                }
            }
        } else {
            self.child.event(ctx, &child_event, data, env);
        }
    }

    #[instrument(name = "ClipBox", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env);
    }

    #[instrument(
        name = "ClipBox",
        level = "trace",
        skip(self, ctx, _old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    #[instrument(name = "ClipBox", level = "trace", skip(self, ctx, bc, data, env))]
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
        let min_child_size = if self.must_fill { bc.min() } else { Size::ZERO };
        let child_bc =
            BoxConstraints::new(min_child_size, Size::new(max_child_width, max_child_height));

        let bc_changed = child_bc != self.old_bc;
        self.old_bc = child_bc;

        let content_size = if bc_changed || self.child.layout_requested() {
            self.child.layout(ctx, &child_bc, data, env)
        } else {
            self.child.layout_rect().size()
        };

        self.port.content_size = content_size;
        self.child.set_origin(ctx, data, env, Point::ORIGIN);

        self.port.view_size = bc.constrain(content_size);
        let new_offset = self.port.clamp_view_origin(self.viewport_origin());
        self.pan_to(new_offset);
        trace!("Computed sized: {}", self.viewport_size());
        self.viewport_size()
    }

    #[instrument(name = "ClipBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.clip(ctx.size().to_rect());
        self.child.paint_raw(ctx, data, env);
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn pan_to_visible() {
        let mut viewport = Viewport {
            content_size: Size::new(400., 400.),
            view_size: (20., 20.).into(),
            view_origin: (20., 20.).into(),
        };

        assert!(!viewport.pan_to_visible(Rect::from_origin_size((22., 22.,), (5., 5.))));
        assert!(viewport.pan_to_visible(Rect::from_origin_size((10., 10.,), (5., 5.))));
        assert_eq!(viewport.view_origin, Point::new(10., 10.));
        assert_eq!(viewport.view_size, Size::new(20., 20.));
        assert!(!viewport.pan_to_visible(Rect::from_origin_size((10., 10.,), (50., 50.))));
        assert_eq!(viewport.view_origin, Point::new(10., 10.));

        assert!(viewport.pan_to_visible(Rect::from_origin_size((30., 10.,), (5., 5.))));
        assert_eq!(viewport.view_origin, Point::new(15., 10.));
        assert!(viewport.pan_to_visible(Rect::from_origin_size((5., 5.,), (5., 5.))));
        assert_eq!(viewport.view_origin, Point::new(5., 5.));
    }
}
