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

//! A component for embedding in another widget to provide consistent and
//! extendable scrolling behavior

use std::time::Duration;

use crate::kurbo::{Affine, Point, Rect, RoundedRect, Size, Vec2};
use crate::theme;
use crate::{
    Env, Event, EventCtx, LifeCycle, LifeCycleCtx, PaintCtx, Region, RenderContext, TimerToken,
};

//TODO: Add this to env
/// Minimum length for any scrollbar to be when measured on that
/// scrollbar's primary axis.
pub const SCROLLBAR_MIN_SIZE: f64 = 45.0;

/// Denotes which scrollbar, if any, is currently being hovered over
/// by the mouse.
#[derive(Debug, Copy, Clone)]
pub enum BarHoveredState {
    /// Neither scrollbar is being hovered by the mouse.
    None,
    /// The vertical scrollbar is being hovered by the mouse.
    Vertical,
    /// The horizontal scrollbar is being hovered by the mouse.
    Horizontal,
}

impl BarHoveredState {
    /// Determines if any scrollbar is currently being hovered by the mouse.
    pub fn is_hovered(self) -> bool {
        matches!(
            self,
            BarHoveredState::Vertical | BarHoveredState::Horizontal
        )
    }
}

/// Denotes which scrollbar, if any, is currently being dragged.
#[derive(Debug, Copy, Clone)]
pub enum BarHeldState {
    /// Neither scrollbar is being dragged.
    None,
    /// Vertical scrollbar is being dragged. Contains an `f64` with
    /// the initial y-offset of the dragging input.
    Vertical(f64),
    /// Horizontal scrollbar is being dragged. Contains an `f64` with
    /// the initial x-offset of the dragging input.
    Horizontal(f64),
}

/// Backing struct for storing scrollbar state
#[derive(Debug, Copy, Clone)]
pub struct ScrollbarsState {
    /// Current opacity for both scrollbars
    pub opacity: f64,
    /// ID for the timer which schedules scrollbar fade out
    pub timer_id: TimerToken,
    /// Which if any scrollbar is currently hovered by the mouse
    pub hovered: BarHoveredState,
    /// Which if any scrollbar is currently being dragged by the mouse
    pub held: BarHeldState,
}

impl Default for ScrollbarsState {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            timer_id: TimerToken::INVALID,
            hovered: BarHoveredState::None,
            held: BarHeldState::None,
        }
    }
}

impl ScrollbarsState {
    /// true if either scrollbar is currently held down/being dragged
    pub fn are_held(&self) -> bool {
        !matches!(self.held, BarHeldState::None)
    }
}

/// Embeddable component exposing reusable scroll handling logic.
///
/// In most situations composing [`Scroll`] or [`List`] is a better idea
/// for general UI construction. However some cases are not covered by
/// composing those widgets, such as when a widget needs fine grained
/// control over its scrolling state or doesn't make sense to exist alone
/// without scrolling behavior.
///
/// `ScrollComponent` contains the unified and consistent scroll logic
/// used by both [`Scroll`] and [`List`]. This can be used to add this
/// logic to a custom widget when the need arises.
///
/// It should be used like this:
/// - Store an instance of `ScrollComponent` in your widget's struct.
/// - During layout, set the [`content_size`] field to the child's size.
/// - Call [`event`] and [`lifecycle`] with all event and lifecycle events before propagating them to children.
/// - Call [`handle_scroll`] with all events after handling / propagating them.
/// - And finally perform painting using the provided [`paint_content`] function.
///
/// Also, taking a look at the [`Scroll`] source code can be helpful.
///
/// [`Scroll`]: ../widget/struct.Scroll.html
/// [`List`]: ../widget/struct.List.html
/// [`content_size`]: struct.ScrollComponent.html#structfield.content_size
/// [`event`]: struct.ScrollComponent.html#method.event
/// [`handle_scroll`]: struct.ScrollComponent.html#method.handle_scroll
/// [`lifecycle`]: struct.ScrollComponent.html#method.lifecycle
/// [`paint_content`]: struct.ScrollComponent.html#method.paint_content
#[derive(Debug, Copy, Clone)]
pub struct ScrollComponent {
    /// The size of the scrollable content, make sure to keep up this
    /// accurate to the content being scrolled
    pub content_size: Size,
    /// Current offset of the scrolling content
    pub scroll_offset: Vec2,
    /// Current state of both scrollbars
    pub scrollbars: ScrollbarsState,
}

impl Default for ScrollComponent {
    fn default() -> Self {
        ScrollComponent::new()
    }
}

impl ScrollComponent {
    /// Constructs a new [`ScrollComponent`](struct.ScrollComponent.html) for use.
    pub fn new() -> ScrollComponent {
        ScrollComponent {
            content_size: Size::default(),
            scroll_offset: Vec2::new(0.0, 0.0),
            scrollbars: ScrollbarsState::default(),
        }
    }

    /// Scroll `delta` units.
    ///
    /// Returns `true` if the scroll offset has changed.
    pub fn scroll(&mut self, delta: Vec2, layout_size: Size) -> bool {
        let mut offset = self.scroll_offset + delta;
        offset.x = offset
            .x
            .min(self.content_size.width - layout_size.width)
            .max(0.0);
        offset.y = offset
            .y
            .min(self.content_size.height - layout_size.height)
            .max(0.0);
        if (offset - self.scroll_offset).hypot2() > 1e-12 {
            self.scroll_offset = offset;
            true
        } else {
            false
        }
    }

    /// Makes the scrollbars visible, and resets the fade timer.
    pub fn reset_scrollbar_fade<F>(&mut self, request_timer: F, env: &Env)
    where
        F: FnOnce(Duration) -> TimerToken,
    {
        self.scrollbars.opacity = env.get(theme::SCROLLBAR_MAX_OPACITY);
        let fade_delay = env.get(theme::SCROLLBAR_FADE_DELAY);
        let deadline = Duration::from_millis(fade_delay);
        self.scrollbars.timer_id = request_timer(deadline);
    }

    /// Calculates the paint rect of the vertical scrollbar.
    ///
    /// Returns `Rect::ZERO` if the vertical scrollbar is not visible.
    pub fn calc_vertical_bar_bounds(&self, viewport: Rect, env: &Env) -> Rect {
        if viewport.height() >= self.content_size.height {
            return Rect::ZERO;
        }

        let bar_width = env.get(theme::SCROLLBAR_WIDTH);
        let bar_pad = env.get(theme::SCROLLBAR_PAD);

        let percent_visible = viewport.height() / self.content_size.height;
        let percent_scrolled =
            self.scroll_offset.y / (self.content_size.height - viewport.height());

        let length = (percent_visible * viewport.height()).ceil();
        let length = length.max(SCROLLBAR_MIN_SIZE);

        let vertical_padding = bar_pad + bar_pad + bar_width;

        let top_y_offset =
            ((viewport.height() - length - vertical_padding) * percent_scrolled).ceil();
        let bottom_y_offset = top_y_offset + length;

        let x0 = self.scroll_offset.x + viewport.width() - bar_width - bar_pad;
        let y0 = self.scroll_offset.y + top_y_offset + bar_pad;

        let x1 = self.scroll_offset.x + viewport.width() - bar_pad;
        let y1 = self.scroll_offset.y + bottom_y_offset;

        Rect::new(x0, y0, x1, y1)
    }

    /// Calculates the paint rect of the horizontal scrollbar.
    ///
    /// Returns `Rect::ZERO` if the horizontal scrollbar is not visible.
    pub fn calc_horizontal_bar_bounds(&self, viewport: Rect, env: &Env) -> Rect {
        if viewport.width() >= self.content_size.width {
            return Rect::ZERO;
        }

        let bar_width = env.get(theme::SCROLLBAR_WIDTH);
        let bar_pad = env.get(theme::SCROLLBAR_PAD);

        let percent_visible = viewport.width() / self.content_size.width;
        let percent_scrolled = self.scroll_offset.x / (self.content_size.width - viewport.width());

        let length = (percent_visible * viewport.width()).ceil();
        let length = length.max(SCROLLBAR_MIN_SIZE);

        let horizontal_padding = bar_pad + bar_pad + bar_width;

        let left_x_offset =
            ((viewport.width() - length - horizontal_padding) * percent_scrolled).ceil();
        let right_x_offset = left_x_offset + length;

        let x0 = self.scroll_offset.x + left_x_offset + bar_pad;
        let y0 = self.scroll_offset.y + viewport.height() - bar_width - bar_pad;

        let x1 = self.scroll_offset.x + right_x_offset;
        let y1 = self.scroll_offset.y + viewport.height() - bar_pad;

        Rect::new(x0, y0, x1, y1)
    }

    /// Draw scroll bars.
    pub fn draw_bars(&self, ctx: &mut PaintCtx, viewport: Rect, env: &Env) {
        if self.scrollbars.opacity <= 0.0 {
            return;
        }

        let brush = ctx.render_ctx.solid_brush(
            env.get(theme::SCROLLBAR_COLOR)
                .with_alpha(self.scrollbars.opacity),
        );
        let border_brush = ctx.render_ctx.solid_brush(
            env.get(theme::SCROLLBAR_BORDER_COLOR)
                .with_alpha(self.scrollbars.opacity),
        );

        let radius = env.get(theme::SCROLLBAR_RADIUS);
        let edge_width = env.get(theme::SCROLLBAR_EDGE_WIDTH);

        // Vertical bar
        if viewport.height() < self.content_size.height {
            let bounds = self
                .calc_vertical_bar_bounds(viewport, env)
                .inset(-edge_width / 2.0);
            let rect = RoundedRect::from_rect(bounds, radius);
            ctx.render_ctx.fill(rect, &brush);
            ctx.render_ctx.stroke(rect, &border_brush, edge_width);
        }

        // Horizontal bar
        if viewport.width() < self.content_size.width {
            let bounds = self
                .calc_horizontal_bar_bounds(viewport, env)
                .inset(-edge_width / 2.0);
            let rect = RoundedRect::from_rect(bounds, radius);
            ctx.render_ctx.fill(rect, &brush);
            ctx.render_ctx.stroke(rect, &border_brush, edge_width);
        }
    }

    /// Tests if the specified point overlaps the vertical scrollbar
    ///
    /// Returns false if the vertical scrollbar is not visible
    pub fn point_hits_vertical_bar(&self, viewport: Rect, pos: Point, env: &Env) -> bool {
        if viewport.height() < self.content_size.height {
            // Stretch hitbox to edge of widget
            let mut bounds = self.calc_vertical_bar_bounds(viewport, env);
            bounds.x1 = self.scroll_offset.x + viewport.width();
            bounds.contains(pos)
        } else {
            false
        }
    }

    /// Tests if the specified point overlaps the horizontal scrollbar
    ///
    /// Returns false if the horizontal scrollbar is not visible
    pub fn point_hits_horizontal_bar(&self, viewport: Rect, pos: Point, env: &Env) -> bool {
        if viewport.width() < self.content_size.width {
            // Stretch hitbox to edge of widget
            let mut bounds = self.calc_horizontal_bar_bounds(viewport, env);
            bounds.y1 = self.scroll_offset.y + viewport.height();
            bounds.contains(pos)
        } else {
            false
        }
    }

    /// Checks if the event applies to the scroll behavior, uses it, and marks it handled
    ///
    /// Make sure to call on every event
    pub fn event(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        let size = ctx.size();
        let viewport = Rect::from_origin_size(Point::ORIGIN, size);

        let scrollbar_is_hovered = match event {
            Event::MouseMove(e) | Event::MouseUp(e) | Event::MouseDown(e) => {
                let offset_pos = e.pos + self.scroll_offset;
                self.point_hits_vertical_bar(viewport, offset_pos, env)
                    || self.point_hits_horizontal_bar(viewport, offset_pos, env)
            }
            _ => false,
        };

        if self.scrollbars.are_held() {
            // if we're dragging a scrollbar
            match event {
                Event::MouseMove(event) => {
                    match self.scrollbars.held {
                        BarHeldState::Vertical(offset) => {
                            let scale_y = viewport.height() / self.content_size.height;
                            let bounds = self.calc_vertical_bar_bounds(viewport, env);
                            let mouse_y = event.pos.y + self.scroll_offset.y;
                            let delta = mouse_y - bounds.y0 - offset;
                            self.scroll(Vec2::new(0f64, (delta / scale_y).ceil()), size);
                            ctx.set_handled();
                        }
                        BarHeldState::Horizontal(offset) => {
                            let scale_x = viewport.width() / self.content_size.width;
                            let bounds = self.calc_horizontal_bar_bounds(viewport, env);
                            let mouse_x = event.pos.x + self.scroll_offset.x;
                            let delta = mouse_x - bounds.x0 - offset;
                            self.scroll(Vec2::new((delta / scale_x).ceil(), 0f64), size);
                            ctx.set_handled();
                        }
                        _ => (),
                    }
                    ctx.request_paint();
                }
                Event::MouseUp(_) => {
                    self.scrollbars.held = BarHeldState::None;
                    ctx.set_active(false);

                    if !scrollbar_is_hovered {
                        self.scrollbars.hovered = BarHoveredState::None;
                        self.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                    }

                    ctx.set_handled();
                }
                _ => (), // other events are a noop
            }
        } else if scrollbar_is_hovered {
            // if we're over a scrollbar but not dragging
            match event {
                Event::MouseMove(event) => {
                    let offset_pos = event.pos + self.scroll_offset;
                    if self.point_hits_vertical_bar(viewport, offset_pos, env) {
                        self.scrollbars.hovered = BarHoveredState::Vertical;
                    } else if self.point_hits_horizontal_bar(viewport, offset_pos, env) {
                        self.scrollbars.hovered = BarHoveredState::Horizontal;
                    } else {
                        unreachable!();
                    }

                    self.scrollbars.opacity = env.get(theme::SCROLLBAR_MAX_OPACITY);
                    self.scrollbars.timer_id = TimerToken::INVALID; // Cancel any fade out in progress
                    ctx.request_paint();
                    ctx.set_handled();
                }
                Event::MouseDown(event) => {
                    let pos = event.pos + self.scroll_offset;

                    if self.point_hits_vertical_bar(viewport, pos, env) {
                        ctx.set_active(true);
                        self.scrollbars.held = BarHeldState::Vertical(
                            pos.y - self.calc_vertical_bar_bounds(viewport, env).y0,
                        );
                    } else if self.point_hits_horizontal_bar(viewport, pos, env) {
                        ctx.set_active(true);
                        self.scrollbars.held = BarHeldState::Horizontal(
                            pos.x - self.calc_horizontal_bar_bounds(viewport, env).x0,
                        );
                    } else {
                        unreachable!();
                    }

                    ctx.set_handled();
                }
                // if the mouse was downed elsewhere, moved over a scroll bar and released: noop.
                Event::MouseUp(_) => (),
                _ => unreachable!(),
            }
        } else {
            match event {
                Event::MouseMove(_) => {
                    // if we have just stopped hovering
                    if self.scrollbars.hovered.is_hovered() && !scrollbar_is_hovered {
                        self.scrollbars.hovered = BarHoveredState::None;
                        self.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                    }
                }
                Event::Timer(id) if *id == self.scrollbars.timer_id => {
                    // Schedule scroll bars animation
                    ctx.request_anim_frame();
                    self.scrollbars.timer_id = TimerToken::INVALID;
                    ctx.set_handled();
                }
                Event::AnimFrame(interval) => {
                    // Guard by the timer id being invalid, otherwise the scroll bars would fade
                    // immediately if some other widget started animating.
                    if self.scrollbars.timer_id == TimerToken::INVALID {
                        // Animate scroll bars opacity
                        let diff = 2.0 * (*interval as f64) * 1e-9;
                        self.scrollbars.opacity -= diff;
                        if self.scrollbars.opacity > 0.0 {
                            ctx.request_anim_frame();
                        }

                        let viewport = ctx.size().to_rect();
                        if viewport.width() < self.content_size.width {
                            ctx.request_paint_rect(
                                self.calc_horizontal_bar_bounds(viewport, env) - self.scroll_offset,
                            );
                        }
                        if viewport.height() < self.content_size.height {
                            ctx.request_paint_rect(
                                self.calc_vertical_bar_bounds(viewport, env) - self.scroll_offset,
                            );
                        }
                    }
                }

                _ => (),
            }
        }
    }

    /// Applies mousewheel scrolling if the event has not already been handled
    pub fn handle_scroll(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        if !ctx.is_handled() {
            if let Event::Wheel(mouse) = event {
                if self.scroll(mouse.wheel_delta, ctx.size()) {
                    ctx.request_paint();
                    ctx.set_handled();
                    self.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                }
            }
        }
    }

    /// Perform any necessary action prompted by a lifecycle event
    ///
    /// Make sure to call on every lifecycle event
    pub fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, env: &Env) {
        if let LifeCycle::Size(_) = event {
            // Show the scrollbars any time our size changes
            self.reset_scrollbar_fade(|d| ctx.request_timer(d), &env);
        }
    }

    /// Helper function to paint a closure at the correct offset with clipping and scrollbars
    pub fn paint_content(
        self,
        ctx: &mut PaintCtx,
        env: &Env,
        f: impl FnOnce(Region, &mut PaintCtx),
    ) {
        let viewport = ctx.size().to_rect();
        ctx.with_save(|ctx| {
            ctx.clip(viewport);
            ctx.transform(Affine::translate(-self.scroll_offset));

            let mut visible = ctx.region().clone();
            visible += self.scroll_offset;
            f(visible, ctx);

            self.draw_bars(ctx, viewport, env);
        });
    }
}
