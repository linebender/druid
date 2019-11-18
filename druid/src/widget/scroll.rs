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
use std::time::{Duration, Instant};

use log::error;

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point, Rect, Size,
    TimerToken, UpdateCtx, Vec2, Widget, WidgetPod,
};

use crate::piet::RenderContext;
use crate::theme;

use crate::kurbo::{Affine, RoundedRect};

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

enum BarHoveredState {
    None,
    Vertical,
    Horizontal,
}

impl BarHoveredState {
    fn is_hovered(&self) -> bool {
        match self {
            BarHoveredState::Vertical | BarHoveredState::Horizontal => true,
            _ => false,
        }
    }
}

enum BarHeldState {
    None,
    /// Vertical scrollbar is being dragged. Contains an `f64` with
    /// the initial y-offset of the dragging input
    Vertical(f64),
    /// Horizontal scrollbar is being dragged. Contains an `f64` with
    /// the initial x-offset of the dragging input
    Horizontal(f64),
}

struct ScrollBarsState {
    opacity: f64,
    timer_id: TimerToken,
    hovered: BarHoveredState,
    held: BarHeldState,
}

impl Default for ScrollBarsState {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            timer_id: TimerToken::INVALID,
            hovered: BarHoveredState::None,
            held: BarHeldState::None,
        }
    }
}

/// A container that scrolls its contents.
///
/// This container holds a single child, and uses the wheel to scroll it
/// when the child's bounds are larger than the viewport.
///
/// The child is laid out with completely unconstrained layout bounds.
pub struct Scroll<T: Data, W: Widget<T>> {
    child: WidgetPod<T, W>,
    child_size: Size,
    scroll_offset: Vec2,
    direction: ScrollDirection,
    scroll_bars: ScrollBarsState,
}

impl<T: Data, W: Widget<T>> Scroll<T, W> {
    /// Create a new scroll container.
    ///
    /// This method will allow scrolling in all directions if child's bounds
    /// are larger than the viewport. Use [vertical](#method.vertical)
    /// and [horizontal](#method.horizontal) methods to limit scroll behavior.
    pub fn new(child: W) -> Scroll<T, W> {
        Scroll {
            child: WidgetPod::new(child),
            child_size: Default::default(),
            scroll_offset: Vec2::new(0.0, 0.0),
            direction: ScrollDirection::All,
            scroll_bars: ScrollBarsState::default(),
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

    /// Returns a reference to the child widget.
    pub fn child(&self) -> &W {
        self.child.widget()
    }

    /// Returns a mutable reference to the child widget.
    pub fn child_mut(&mut self) -> &mut W {
        self.child.widget_mut()
    }

    /// Update the scroll.
    ///
    /// Returns `true` if the scroll has been updated.
    pub fn scroll(&mut self, delta: Vec2, size: Size) -> bool {
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

    /// Makes the scrollbars visible, and resets the fade timer.
    pub fn reset_scrollbar_fade(&mut self, ctx: &mut EventCtx, env: &Env) {
        // Display scroll bars and schedule their disappearance
        self.scroll_bars.opacity = env.get(theme::SCROLL_BAR_MAX_OPACITY);
        let fade_delay = env.get(theme::SCROLL_BAR_FADE_DELAY);
        let deadline = Instant::now() + Duration::from_millis(fade_delay);
        self.scroll_bars.timer_id = ctx.request_timer(deadline);
    }

    /// Returns the current scroll offset.
    pub fn offset(&self) -> Vec2 {
        self.scroll_offset
    }

    fn calc_vertical_bar_bounds(&self, viewport: Rect, env: &Env) -> Rect {
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);
        let bar_pad = env.get(theme::SCROLL_BAR_PAD);

        let scale_y = viewport.height() / self.child_size.height;

        let top_y_offset = (scale_y * self.scroll_offset.y).ceil();
        let bottom_y_offset = (scale_y * viewport.height()).ceil() + top_y_offset;

        let x0 = self.scroll_offset.x + viewport.width() - bar_width - bar_pad;
        let y0 = self.scroll_offset.y + top_y_offset + bar_pad;

        let x1 = self.scroll_offset.x + viewport.width() - bar_pad;
        let y1 = self.scroll_offset.y + bottom_y_offset - (bar_pad * 2.) - bar_width;

        Rect::new(x0, y0, x1, y1)
    }

    fn calc_horizontal_bar_bounds(&self, viewport: Rect, env: &Env) -> Rect {
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);
        let bar_pad = env.get(theme::SCROLL_BAR_PAD);

        let scale_x = viewport.width() / self.child_size.width;

        let left_x_offset = (scale_x * self.scroll_offset.x).ceil();
        let right_x_offset = (scale_x * viewport.width()).ceil() + left_x_offset;

        let x0 = self.scroll_offset.x + left_x_offset + bar_pad;
        let y0 = self.scroll_offset.y + viewport.height() - bar_width - bar_pad;

        let x1 = self.scroll_offset.x + right_x_offset - (bar_pad * 2.) - bar_width;
        let y1 = self.scroll_offset.y + viewport.height() - bar_pad;

        Rect::new(x0, y0, x1, y1)
    }

    /// Draw scroll bars.
    fn draw_bars(&self, paint_ctx: &mut PaintCtx, viewport: Rect, env: &Env) {
        if self.scroll_bars.opacity <= 0.0 {
            return;
        }

        let brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_COLOR)
                .with_alpha(self.scroll_bars.opacity),
        );
        let border_brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_BORDER_COLOR)
                .with_alpha(self.scroll_bars.opacity),
        );

        let radius = env.get(theme::SCROLL_BAR_RADIUS);
        let edge_width = env.get(theme::SCROLL_BAR_EDGE_WIDTH);

        // Vertical bar
        if viewport.height() < self.child_size.height {
            let bounds = self.calc_vertical_bar_bounds(viewport, &env);
            let rect = RoundedRect::from_rect(bounds, radius);
            paint_ctx.render_ctx.fill(rect, &brush);
            paint_ctx.render_ctx.stroke(rect, &border_brush, edge_width);
        }

        // Horizontal bar
        if viewport.width() < self.child_size.width {
            let bounds = self.calc_horizontal_bar_bounds(viewport, &env);
            let rect = RoundedRect::from_rect(bounds, radius);
            paint_ctx.render_ctx.fill(rect, &brush);
            paint_ctx.render_ctx.stroke(rect, &border_brush, edge_width);
        }
    }

    fn point_hits_vertical_bar(&self, viewport: Rect, pos: Point, env: &Env) -> bool {
        if viewport.height() < self.child_size.height {
            let bounds = self.calc_vertical_bar_bounds(viewport, &env);
            return pos.y > bounds.y0 && pos.y < bounds.y1 && pos.x > bounds.x0;
        }

        false
    }

    fn point_hits_horizontal_bar(&self, viewport: Rect, pos: Point, env: &Env) -> bool {
        if viewport.width() < self.child_size.width {
            let bounds = self.calc_horizontal_bar_bounds(viewport, &env);
            return pos.x > bounds.x0 && pos.x < bounds.x1 && pos.y > bounds.y0;
        }

        false
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Scroll<T, W> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        if let Err(e) = paint_ctx.save() {
            error!("saving render context failed: {:?}", e);
            return;
        }
        let viewport = Rect::from_origin_size(Point::ORIGIN, base_state.size());
        paint_ctx.clip(viewport);
        paint_ctx.transform(Affine::translate(-self.scroll_offset));

        let visible = viewport.with_origin(self.scroll_offset.to_point());
        paint_ctx.with_child_ctx(visible, |ctx| self.child.paint(ctx, data, env));

        self.draw_bars(paint_ctx, viewport, env);

        if let Err(e) = paint_ctx.restore() {
            error!("restoring render context failed: {:?}", e);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Scroll");

        let child_bc = BoxConstraints::new(Size::ZERO, self.direction.max_size(bc));
        let size = self.child.layout(ctx, &child_bc, data, env);
        self.child_size = size;
        self.child
            .set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));
        let self_size = bc.constrain(Size::new(100.0, 100.0));
        let _ = self.scroll(Vec2::new(0.0, 0.0), self_size);
        self_size
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let size = ctx.base_state.size();
        let viewport = Rect::from_origin_size(Point::ORIGIN, size);

        if match event {
            Event::MouseMoved(_) | Event::MouseUp(_) => match self.scroll_bars.held {
                BarHeldState::Vertical(_) | BarHeldState::Horizontal(_) => true,
                _ => false,
            },
            _ => false,
        } {
            match event {
                Event::MouseMoved(event) => {
                    match self.scroll_bars.held {
                        BarHeldState::Vertical(offset) => {
                            let scale_y = viewport.height() / self.child_size.height;
                            let bounds = self.calc_vertical_bar_bounds(viewport, &env);
                            let mouse_y = event.pos.y + self.scroll_offset.y;
                            let delta = mouse_y - bounds.y0 - offset;
                            self.scroll(Vec2::new(0f64, (delta / scale_y).ceil()), size);
                        }
                        BarHeldState::Horizontal(offset) => {
                            let scale_x = viewport.width() / self.child_size.width;
                            let bounds = self.calc_horizontal_bar_bounds(viewport, &env);
                            let mouse_x = event.pos.x + self.scroll_offset.x;
                            let delta = mouse_x - bounds.x0 - offset;
                            self.scroll(Vec2::new((delta / scale_x).ceil(), 0f64), size);
                        }
                        _ => (),
                    }
                    ctx.invalidate();
                }
                Event::MouseUp(_) => {
                    self.scroll_bars.held = BarHeldState::None;
                }
                _ => (),
            }
        } else if match event {
            Event::MouseMoved(event) | Event::MouseDown(event) => {
                let mut transformed_event = event.clone();
                transformed_event.pos += self.scroll_offset;
                self.point_hits_vertical_bar(viewport, transformed_event.pos, &env)
                    || self.point_hits_horizontal_bar(viewport, transformed_event.pos, &env)
            }
            _ => false,
        } {
            match event {
                Event::MouseMoved(event) => {
                    let mut transformed_event = event.clone();
                    transformed_event.pos += self.scroll_offset;
                    if self.point_hits_vertical_bar(viewport, transformed_event.pos, &env) {
                        self.scroll_bars.hovered = BarHoveredState::Vertical;
                    } else {
                        self.scroll_bars.hovered = BarHoveredState::Horizontal;
                    }

                    self.scroll_bars.opacity = env.get(theme::SCROLL_BAR_MAX_OPACITY);
                    self.scroll_bars.timer_id = TimerToken::INVALID; // Cancel any fade out in progress
                    ctx.invalidate();
                }
                Event::MouseDown(event) => {
                    let pos = event.pos + self.scroll_offset;

                    if self.point_hits_vertical_bar(viewport, pos, &env) {
                        self.scroll_bars.held = BarHeldState::Vertical(
                            pos.y - self.calc_vertical_bar_bounds(viewport, &env).y0,
                        );
                    } else if self.point_hits_horizontal_bar(viewport, pos, &env) {
                        self.scroll_bars.held = BarHeldState::Horizontal(
                            pos.x - self.calc_horizontal_bar_bounds(viewport, &env).x0,
                        );
                    }
                }
                _ => (),
            }
        } else {
            let child_event = event.transform_scroll(self.scroll_offset, viewport);
            if let Some(child_event) = child_event {
                self.child.event(ctx, &child_event, data, env)
            };

            match event {
                Event::MouseMoved(event) => {
                    let mut transformed_event = event.clone();
                    transformed_event.pos += self.scroll_offset;
                    let pos = transformed_event.pos;
                    let hits_vertical = self.point_hits_vertical_bar(viewport, pos, &env);
                    let hits_horizontal = self.point_hits_horizontal_bar(viewport, pos, &env);
                    let currently_hovered = hits_vertical || hits_horizontal;
                    if self.scroll_bars.hovered.is_hovered() && !currently_hovered {
                        self.scroll_bars.hovered = BarHoveredState::None;
                        self.reset_scrollbar_fade(ctx, &env);
                    }
                }
                // Show the scrollbars any time our size changes
                Event::Size(_) => self.reset_scrollbar_fade(ctx, &env),
                // The scroll bars will fade immediately if there's some other widget requesting animation.
                // Guard by the timer id being invalid.
                Event::AnimFrame(interval) if self.scroll_bars.timer_id == TimerToken::INVALID => {
                    // Animate scroll bars opacity
                    let diff = 2.0 * (*interval as f64) * 1e-9;
                    self.scroll_bars.opacity -= diff;
                    if self.scroll_bars.opacity > 0.0 {
                        ctx.request_anim_frame();
                    }
                }
                Event::Timer(id) if *id == self.scroll_bars.timer_id => {
                    // Schedule scroll bars animation
                    ctx.request_anim_frame();
                    self.scroll_bars.timer_id = TimerToken::INVALID;
                }
                _ => (),
            }
        }

        if !ctx.is_handled() {
            if let Event::Wheel(wheel) = event {
                if self.scroll(wheel.delta, size) {
                    ctx.invalidate();
                    ctx.set_handled();
                    self.reset_scrollbar_fade(ctx, &env);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }
}
