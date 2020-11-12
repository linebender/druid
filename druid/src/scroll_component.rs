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

use crate::kurbo::{Point, Rect, Vec2};
use crate::theme;
use crate::widget::Viewport;
use crate::{Env, Event, EventCtx, LifeCycle, LifeCycleCtx, PaintCtx, RenderContext, TimerToken};

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

/// Embeddable component exposing reusable scroll handling logic.
///
/// In most situations composing [`Scroll`] is a better idea
/// for general UI construction. However some cases are not covered by
/// composing those widgets, such as when a widget needs fine grained
/// control over its scrolling state or doesn't make sense to exist alone
/// without scrolling behavior.
///
/// `ScrollComponent` contains the input-handling and scrollbar-positioning logic used by
/// [`Scroll`].  It can be used to add this logic to a custom widget when the need arises.
///
/// It can be used like this:
/// - Store an instance of `ScrollComponent` in your widget's struct, and wrap the child widget to
///   be scrolled in a [`ClipBox`].
/// - Call [`event`] and [`lifecycle`] with all event and lifecycle events before propagating them
///   to children.
/// - Call [`handle_scroll`] with all events after handling / propagating them.
/// - Call [`draw_bars`] to draw the scrollbars.
///
/// Taking a look at the [`Scroll`] source code can be helpful. You can also do scrolling
/// without wrapping a child in a [`ClipBox`], but you will need to do certain event and
/// paint transformations yourself; see the [`ClipBox`] source code for an example.
///
/// [`Scroll`]: ../widget/struct.Scroll.html
/// [`List`]: ../widget/struct.List.html
/// [`ClipBox`]: ../widget/struct.ClipBox.html
/// [`event`]: struct.ScrollComponent.html#method.event
/// [`handle_scroll`]: struct.ScrollComponent.html#method.handle_scroll
/// [`draw_bars`]: #method.draw_bars
/// [`lifecycle`]: struct.ScrollComponent.html#method.lifecycle
#[derive(Debug, Copy, Clone)]
pub struct ScrollComponent {
    /// Current opacity for both scrollbars
    pub opacity: f64,
    /// ID for the timer which schedules scrollbar fade out
    pub timer_id: TimerToken,
    /// Which if any scrollbar is currently hovered by the mouse
    pub hovered: BarHoveredState,
    /// Which if any scrollbar is currently being dragged by the mouse
    pub held: BarHeldState,
}

impl Default for ScrollComponent {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            timer_id: TimerToken::INVALID,
            hovered: BarHoveredState::None,
            held: BarHeldState::None,
        }
    }
}

impl ScrollComponent {
    /// Constructs a new [`ScrollComponent`](struct.ScrollComponent.html) for use.
    pub fn new() -> ScrollComponent {
        Default::default()
    }

    /// true if either scrollbar is currently held down/being dragged
    pub fn are_bars_held(&self) -> bool {
        !matches!(self.held, BarHeldState::None)
    }

    /// Makes the scrollbars visible, and resets the fade timer.
    pub fn reset_scrollbar_fade<F>(&mut self, request_timer: F, env: &Env)
    where
        F: FnOnce(Duration) -> TimerToken,
    {
        self.opacity = env.get(theme::SCROLLBAR_MAX_OPACITY);
        let fade_delay = env.get(theme::SCROLLBAR_FADE_DELAY);
        let deadline = Duration::from_millis(fade_delay);
        self.timer_id = request_timer(deadline);
    }

    /// Calculates the paint rect of the vertical scrollbar, or `None` if the vertical scrollbar is
    /// not visible.
    pub fn calc_vertical_bar_bounds(&self, port: &Viewport, env: &Env) -> Option<Rect> {
        let viewport_size = port.rect.size();
        let content_size = port.content_size;
        let scroll_offset = port.rect.origin().to_vec2();

        if viewport_size.height >= content_size.height {
            return None;
        }

        let bar_width = env.get(theme::SCROLLBAR_WIDTH);
        let bar_pad = env.get(theme::SCROLLBAR_PAD);

        let percent_visible = viewport_size.height / content_size.height;
        let percent_scrolled = scroll_offset.y / (content_size.height - viewport_size.height);

        let length = (percent_visible * viewport_size.height).ceil();
        let length = length.max(SCROLLBAR_MIN_SIZE);

        let vertical_padding = bar_pad + bar_pad + bar_width;

        let top_y_offset =
            ((viewport_size.height - length - vertical_padding) * percent_scrolled).ceil();
        let bottom_y_offset = top_y_offset + length;

        let x0 = scroll_offset.x + viewport_size.width - bar_width - bar_pad;
        let y0 = scroll_offset.y + top_y_offset + bar_pad;

        let x1 = scroll_offset.x + viewport_size.width - bar_pad;
        let y1 = scroll_offset.y + bottom_y_offset;

        Some(Rect::new(x0, y0, x1, y1))
    }

    /// Calculates the paint rect of the horizontal scrollbar, or `None` if the horizontal
    /// scrollbar is not visible.
    pub fn calc_horizontal_bar_bounds(&self, port: &Viewport, env: &Env) -> Option<Rect> {
        let viewport_size = port.rect.size();
        let content_size = port.content_size;
        let scroll_offset = port.rect.origin().to_vec2();

        if viewport_size.width >= content_size.width {
            return None;
        }

        let bar_width = env.get(theme::SCROLLBAR_WIDTH);
        let bar_pad = env.get(theme::SCROLLBAR_PAD);

        let percent_visible = viewport_size.width / content_size.width;
        let percent_scrolled = scroll_offset.x / (content_size.width - viewport_size.width);

        let length = (percent_visible * viewport_size.width).ceil();
        let length = length.max(SCROLLBAR_MIN_SIZE);

        let horizontal_padding = bar_pad + bar_pad + bar_width;

        let left_x_offset =
            ((viewport_size.width - length - horizontal_padding) * percent_scrolled).ceil();
        let right_x_offset = left_x_offset + length;

        let x0 = scroll_offset.x + left_x_offset + bar_pad;
        let y0 = scroll_offset.y + viewport_size.height - bar_width - bar_pad;

        let x1 = scroll_offset.x + right_x_offset;
        let y1 = scroll_offset.y + viewport_size.height - bar_pad;

        Some(Rect::new(x0, y0, x1, y1))
    }

    /// Draw scroll bars.
    pub fn draw_bars(&self, ctx: &mut PaintCtx, port: &Viewport, env: &Env) {
        let scroll_offset = port.rect.origin().to_vec2();
        if self.opacity <= 0.0 {
            return;
        }

        let brush = ctx
            .render_ctx
            .solid_brush(env.get(theme::SCROLLBAR_COLOR).with_alpha(self.opacity));
        let border_brush = ctx.render_ctx.solid_brush(
            env.get(theme::SCROLLBAR_BORDER_COLOR)
                .with_alpha(self.opacity),
        );

        let radius = env.get(theme::SCROLLBAR_RADIUS);
        let edge_width = env.get(theme::SCROLLBAR_EDGE_WIDTH);

        // Vertical bar
        if let Some(bounds) = self.calc_vertical_bar_bounds(port, env) {
            let rect = (bounds - scroll_offset)
                .inset(-edge_width / 2.0)
                .to_rounded_rect(radius);
            ctx.render_ctx.fill(rect, &brush);
            ctx.render_ctx.stroke(rect, &border_brush, edge_width);
        }

        // Horizontal bar
        if let Some(bounds) = self.calc_horizontal_bar_bounds(port, env) {
            let rect = (bounds - scroll_offset)
                .inset(-edge_width / 2.0)
                .to_rounded_rect(radius);
            ctx.render_ctx.fill(rect, &brush);
            ctx.render_ctx.stroke(rect, &border_brush, edge_width);
        }
    }

    /// Tests if the specified point overlaps the vertical scrollbar
    ///
    /// Returns false if the vertical scrollbar is not visible
    pub fn point_hits_vertical_bar(&self, port: &Viewport, pos: Point, env: &Env) -> bool {
        let viewport_size = port.rect.size();
        let scroll_offset = port.rect.origin().to_vec2();

        if let Some(mut bounds) = self.calc_vertical_bar_bounds(port, env) {
            // Stretch hitbox to edge of widget
            bounds.x1 = scroll_offset.x + viewport_size.width;
            bounds.contains(pos)
        } else {
            false
        }
    }

    /// Tests if the specified point overlaps the horizontal scrollbar
    ///
    /// Returns false if the horizontal scrollbar is not visible
    pub fn point_hits_horizontal_bar(&self, port: &Viewport, pos: Point, env: &Env) -> bool {
        let viewport_size = port.rect.size();
        let scroll_offset = port.rect.origin().to_vec2();

        if let Some(mut bounds) = self.calc_horizontal_bar_bounds(port, env) {
            // Stretch hitbox to edge of widget
            bounds.y1 = scroll_offset.y + viewport_size.height;
            bounds.contains(pos)
        } else {
            false
        }
    }

    /// Checks if the event applies to the scroll behavior, uses it, and marks it handled
    ///
    /// Make sure to call on every event
    pub fn event(&mut self, port: &mut Viewport, ctx: &mut EventCtx, event: &Event, env: &Env) {
        let viewport_size = port.rect.size();
        let content_size = port.content_size;
        let scroll_offset = port.rect.origin().to_vec2();

        let scrollbar_is_hovered = match event {
            Event::MouseMove(e) | Event::MouseUp(e) | Event::MouseDown(e) => {
                let offset_pos = e.pos + scroll_offset;
                self.point_hits_vertical_bar(port, offset_pos, env)
                    || self.point_hits_horizontal_bar(port, offset_pos, env)
            }
            _ => false,
        };

        if self.are_bars_held() {
            // if we're dragging a scrollbar
            match event {
                Event::MouseMove(event) => {
                    match self.held {
                        BarHeldState::Vertical(offset) => {
                            let scale_y = viewport_size.height / content_size.height;
                            let bounds = self
                                .calc_vertical_bar_bounds(port, env)
                                .unwrap_or(Rect::ZERO);
                            let mouse_y = event.pos.y + scroll_offset.y;
                            let delta = mouse_y - bounds.y0 - offset;
                            port.pan_by(Vec2::new(0f64, (delta / scale_y).ceil()));
                            ctx.set_handled();
                        }
                        BarHeldState::Horizontal(offset) => {
                            let scale_x = viewport_size.height / content_size.width;
                            let bounds = self
                                .calc_horizontal_bar_bounds(port, env)
                                .unwrap_or(Rect::ZERO);
                            let mouse_x = event.pos.x + scroll_offset.x;
                            let delta = mouse_x - bounds.x0 - offset;
                            port.pan_by(Vec2::new((delta / scale_x).ceil(), 0f64));
                            ctx.set_handled();
                        }
                        _ => (),
                    }
                    ctx.request_paint();
                }
                Event::MouseUp(_) => {
                    self.held = BarHeldState::None;
                    ctx.set_active(false);

                    if !scrollbar_is_hovered {
                        self.hovered = BarHoveredState::None;
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
                    let offset_pos = event.pos + scroll_offset;
                    if self.point_hits_vertical_bar(port, offset_pos, env) {
                        self.hovered = BarHoveredState::Vertical;
                    } else if self.point_hits_horizontal_bar(port, offset_pos, env) {
                        self.hovered = BarHoveredState::Horizontal;
                    } else {
                        unreachable!();
                    }

                    self.opacity = env.get(theme::SCROLLBAR_MAX_OPACITY);
                    self.timer_id = TimerToken::INVALID; // Cancel any fade out in progress
                    ctx.request_paint();
                    ctx.set_handled();
                }
                Event::MouseDown(event) => {
                    let pos = event.pos + scroll_offset;

                    if self.point_hits_vertical_bar(port, pos, env) {
                        ctx.set_active(true);
                        self.held = BarHeldState::Vertical(
                            // The bounds must be non-empty, because the point hits the scrollbar.
                            pos.y - self.calc_vertical_bar_bounds(port, env).unwrap().y0,
                        );
                    } else if self.point_hits_horizontal_bar(port, pos, env) {
                        ctx.set_active(true);
                        self.held = BarHeldState::Horizontal(
                            // The bounds must be non-empty, because the point hits the scrollbar.
                            pos.x - self.calc_horizontal_bar_bounds(port, env).unwrap().x0,
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
                    if self.hovered.is_hovered() && !scrollbar_is_hovered {
                        self.hovered = BarHoveredState::None;
                        self.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                    }
                }
                Event::Timer(id) if *id == self.timer_id => {
                    // Schedule scroll bars animation
                    ctx.request_anim_frame();
                    self.timer_id = TimerToken::INVALID;
                    ctx.set_handled();
                }
                Event::AnimFrame(interval) => {
                    // Guard by the timer id being invalid, otherwise the scroll bars would fade
                    // immediately if some other widget started animating.
                    if self.timer_id == TimerToken::INVALID {
                        // Animate scroll bars opacity
                        let diff = 2.0 * (*interval as f64) * 1e-9;
                        self.opacity -= diff;
                        if self.opacity > 0.0 {
                            ctx.request_anim_frame();
                        }

                        if let Some(bounds) = self.calc_horizontal_bar_bounds(port, env) {
                            ctx.request_paint_rect(bounds - scroll_offset);
                        }
                        if let Some(bounds) = self.calc_vertical_bar_bounds(port, env) {
                            ctx.request_paint_rect(bounds - scroll_offset);
                        }
                    }
                }

                _ => (),
            }
        }
    }

    /// Applies mousewheel scrolling if the event has not already been handled
    pub fn handle_scroll(
        &mut self,
        port: &mut Viewport,
        ctx: &mut EventCtx,
        event: &Event,
        env: &Env,
    ) {
        if !ctx.is_handled() {
            if let Event::Wheel(mouse) = event {
                if port.pan_by(mouse.wheel_delta) {
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
}
