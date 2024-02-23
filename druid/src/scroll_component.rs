// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A component for embedding in another widget to provide consistent and
//! extendable scrolling behavior

use std::time::Duration;

use crate::kurbo::{Point, Rect, Vec2};
use crate::theme;
use crate::widget::{Axis, Viewport};
use crate::{Env, Event, EventCtx, LifeCycle, LifeCycleCtx, PaintCtx, RenderContext, TimerToken};

#[derive(Default, Debug, Copy, Clone)]
/// Which scroll bars of a scroll area are currently enabled.
pub enum ScrollbarsEnabled {
    /// No scrollbars are enabled
    None,
    /// Scrolling on the x axis is allowed
    Horizontal,
    /// Scrolling on the y axis is allowed
    Vertical,
    /// Bidirectional scrolling is allowed
    #[default]
    Both,
}

impl ScrollbarsEnabled {
    fn is_enabled(self, axis: Axis) -> bool {
        matches!(
            (self, axis),
            (ScrollbarsEnabled::Both, _)
                | (ScrollbarsEnabled::Horizontal, Axis::Horizontal)
                | (ScrollbarsEnabled::Vertical, Axis::Vertical)
        )
    }

    fn is_none(self) -> bool {
        matches!(self, ScrollbarsEnabled::None)
    }

    /// Set whether the horizontal scrollbar is enabled.
    pub fn set_horizontal_scrollbar_enabled(&mut self, enabled: bool) {
        *self = match (*self, enabled) {
            (ScrollbarsEnabled::None, true) | (ScrollbarsEnabled::Horizontal, true) => {
                ScrollbarsEnabled::Horizontal
            }
            (ScrollbarsEnabled::Both, true) | (ScrollbarsEnabled::Vertical, true) => {
                ScrollbarsEnabled::Both
            }
            (ScrollbarsEnabled::None, false) | (ScrollbarsEnabled::Horizontal, false) => {
                ScrollbarsEnabled::None
            }
            (ScrollbarsEnabled::Vertical, false) | (ScrollbarsEnabled::Both, false) => {
                ScrollbarsEnabled::Vertical
            }
        }
    }

    /// Set whether the vertical scrollbar is enabled.
    pub fn set_vertical_scrollbar_enabled(&mut self, enabled: bool) {
        *self = match (*self, enabled) {
            (ScrollbarsEnabled::None, true) | (ScrollbarsEnabled::Vertical, true) => {
                ScrollbarsEnabled::Vertical
            }
            (ScrollbarsEnabled::Both, true) | (ScrollbarsEnabled::Horizontal, true) => {
                ScrollbarsEnabled::Both
            }
            (ScrollbarsEnabled::None, false) | (ScrollbarsEnabled::Vertical, false) => {
                ScrollbarsEnabled::None
            }
            (ScrollbarsEnabled::Horizontal, false) | (ScrollbarsEnabled::Both, false) => {
                ScrollbarsEnabled::Horizontal
            }
        }
    }
}

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
    /// Which scrollbars are enabled
    pub enabled: ScrollbarsEnabled,
}

impl Default for ScrollComponent {
    fn default() -> Self {
        Self {
            opacity: 0.0,
            timer_id: TimerToken::INVALID,
            hovered: BarHoveredState::None,
            held: BarHeldState::None,
            enabled: ScrollbarsEnabled::Both,
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
        self.calc_bar_bounds(Axis::Vertical, port, env)
    }

    /// Calculates the paint rect of the horizontal scrollbar, or `None` if the horizontal
    /// scrollbar is not visible.
    pub fn calc_horizontal_bar_bounds(&self, port: &Viewport, env: &Env) -> Option<Rect> {
        self.calc_bar_bounds(Axis::Horizontal, port, env)
    }

    fn calc_bar_bounds(&self, axis: Axis, port: &Viewport, env: &Env) -> Option<Rect> {
        let viewport_size = port.view_size;
        let content_size = port.content_size;
        let scroll_offset = port.view_origin.to_vec2();

        let viewport_major = axis.major(viewport_size);
        let content_major = axis.major(content_size);

        if viewport_major >= content_major {
            return None;
        }

        let bar_width = env.get(theme::SCROLLBAR_WIDTH);
        let bar_pad = env.get(theme::SCROLLBAR_PAD);
        let bar_min_size = env.get(theme::SCROLLBAR_MIN_SIZE);

        let percent_visible = viewport_major / content_major;
        let percent_scrolled = axis.major_vec(scroll_offset) / (content_major - viewport_major);

        let major_padding = if self.enabled.is_enabled(axis.cross()) {
            bar_pad + bar_pad + bar_width
        } else {
            bar_pad + bar_pad
        };
        let usable_space = viewport_major - major_padding;

        let length = (percent_visible * viewport_major).ceil();
        #[allow(clippy::manual_clamp)] // Usable space could be below the minimum bar size.
        let length = length.max(bar_min_size).min(usable_space);

        let left_x_offset = bar_pad + ((usable_space - length) * percent_scrolled).ceil();
        let right_x_offset = left_x_offset + length;

        let (x0, y0) = axis.pack(
            left_x_offset,
            axis.minor(viewport_size) - bar_width - bar_pad,
        );

        let (x1, y1) = axis.pack(right_x_offset, axis.minor(viewport_size) - bar_pad);

        if x0 >= x1 || y0 >= y1 {
            return None;
        }

        Some(Rect::new(x0, y0, x1, y1) + scroll_offset)
    }

    /// Draw scroll bars.
    pub fn draw_bars(&self, ctx: &mut PaintCtx, port: &Viewport, env: &Env) {
        let scroll_offset = port.view_origin.to_vec2();

        if self.enabled.is_none() || self.opacity <= 0.0 {
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
        if self.enabled.is_enabled(Axis::Vertical) {
            if let Some(bounds) = self.calc_vertical_bar_bounds(port, env) {
                let rect = (bounds - scroll_offset)
                    .inset(-edge_width / 2.0)
                    .to_rounded_rect(radius);
                ctx.render_ctx.fill(rect, &brush);
                ctx.render_ctx.stroke(rect, &border_brush, edge_width);
            }
        }

        // Horizontal bar
        if self.enabled.is_enabled(Axis::Horizontal) {
            if let Some(bounds) = self.calc_horizontal_bar_bounds(port, env) {
                let rect = (bounds - scroll_offset)
                    .inset(-edge_width / 2.0)
                    .to_rounded_rect(radius);
                ctx.render_ctx.fill(rect, &brush);
                ctx.render_ctx.stroke(rect, &border_brush, edge_width);
            }
        }
    }

    /// Tests if the specified point overlaps the vertical scrollbar
    ///
    /// Returns false if the vertical scrollbar is not visible
    pub fn point_hits_vertical_bar(&self, port: &Viewport, pos: Point, env: &Env) -> bool {
        if !self.enabled.is_enabled(Axis::Vertical) {
            return false;
        }
        let viewport_size = port.view_size;
        let scroll_offset = port.view_origin.to_vec2();

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
        if !self.enabled.is_enabled(Axis::Horizontal) {
            return false;
        }
        let viewport_size = port.view_size;
        let scroll_offset = port.view_origin.to_vec2();

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
        let viewport_size = port.view_size;
        let content_size = port.content_size;
        let scroll_offset = port.view_origin.to_vec2();

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
                            let scale_x = viewport_size.width / content_size.width;
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
        match event {
            LifeCycle::Size(_) => {
                // Show the scrollbars any time our size changes
                self.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
            }
            LifeCycle::HotChanged(false) => {
                if self.hovered.is_hovered() {
                    self.hovered = BarHoveredState::None;
                    self.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                }
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use float_cmp::assert_approx_eq;

    use super::*;
    use crate::kurbo::Size;

    const TEST_SCROLLBAR_WIDTH: f64 = 11.0;
    const TEST_SCROLLBAR_PAD: f64 = 3.0;
    const TEST_SCROLLBAR_MIN_SIZE: f64 = 17.0;

    #[test]
    fn scrollbar_layout() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: (0.0, 25.0).into(),
            view_size: (100.0, 50.0).into(),
        };

        let scrollbar_rect = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert!(
            rect_contains(
                viewport.view_rect().inset(TEST_SCROLLBAR_PAD),
                scrollbar_rect
            ),
            "scrollbar should be contained by viewport"
        );
        assert_eq!(scrollbar_rect, Rect::new(86.0, 38.0, 97.0, 63.0));
    }

    #[test]
    fn scrollbar_layout_at_start() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: Point::ZERO,
            view_size: (100.0, 50.0).into(),
        };

        let scrollbar_rect = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert!(
            rect_contains(
                viewport.view_rect().inset(TEST_SCROLLBAR_PAD),
                scrollbar_rect
            ),
            "scrollbar should be contained by viewport"
        );
        // scrollbar should be at start of viewport
        assert_approx_eq!(
            f64,
            scrollbar_rect.y0,
            viewport.view_rect().y0 + TEST_SCROLLBAR_PAD
        );
        assert_eq!(scrollbar_rect, Rect::new(86.0, 3.0, 97.0, 28.0));
    }

    #[test]
    fn scrollbar_layout_at_end() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: (0.0, 50.0).into(),
            view_size: (100.0, 50.0).into(),
        };

        let scrollbar_rect = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert!(
            rect_contains(
                viewport.view_rect().inset(TEST_SCROLLBAR_PAD),
                scrollbar_rect
            ),
            "scrollbar should be contained by viewport"
        );
        // scrollbar should be at end of viewport
        assert_approx_eq!(
            f64,
            scrollbar_rect.y1,
            viewport.view_rect().y1 - TEST_SCROLLBAR_PAD
        );
        assert_eq!(scrollbar_rect, Rect::new(86.0, 72.0, 97.0, 97.0));
    }

    #[test]
    fn scrollbar_layout_change_viewport_position() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let mut viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: (0.0, 25.0).into(),
            view_size: (100.0, 50.0).into(),
        };

        let scrollbar_rect_1 = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        viewport.view_origin += Vec2::new(0.0, 15.0);

        let scrollbar_rect_2 = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert_eq!(
            scrollbar_rect_1.size(),
            scrollbar_rect_2.size(),
            "moving the viewport should not change scrollbar size"
        );
    }

    #[test]
    fn scrollbar_layout_padding_for_other_bar() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Both;
        let viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: (0.0, 50.0).into(),
            view_size: (100.0, 50.0).into(),
        };

        let scrollbar_rect = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert!(
            rect_contains(
                viewport.view_rect().inset(TEST_SCROLLBAR_PAD),
                scrollbar_rect
            ),
            "scrollbar should be contained by viewport"
        );
        assert!(
            scrollbar_rect.y1 + TEST_SCROLLBAR_WIDTH <= viewport.view_rect().y1,
            "vertical scrollbar should leave space for the horizontal scrollbar when both enabled"
        );
        assert_eq!(scrollbar_rect, Rect::new(86.0, 61.0, 97.0, 86.0));
    }

    #[test]
    fn scrollbar_layout_min_bar_size() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let viewport = Viewport {
            content_size: Size::new(100.0, 1000.0),
            view_origin: (0.0, 25.0).into(),
            view_size: (100.0, 50.0).into(),
        };

        let scrollbar_rect = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert!(
            rect_contains(
                viewport.view_rect().inset(TEST_SCROLLBAR_PAD),
                scrollbar_rect
            ),
            "scrollbar should be contained by viewport"
        );
        // scrollbar should use SCROLLBAR_MIN_SIZE when content is much bigger than viewport
        assert_approx_eq!(f64, scrollbar_rect.height(), TEST_SCROLLBAR_MIN_SIZE);
        assert_eq!(scrollbar_rect, Rect::new(86.0, 29.0, 97.0, 46.0));
    }

    #[test]
    fn scrollbar_layout_viewport_too_small_for_min_bar_size() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: (0.0, 25.0).into(),
            view_size: (100.0, 10.0).into(),
        };

        let scrollbar_rect = scroll_component
            .calc_vertical_bar_bounds(&viewport, &test_env())
            .unwrap();

        assert!(
            rect_contains(
                viewport.view_rect().inset(TEST_SCROLLBAR_PAD),
                scrollbar_rect
            ),
            "scrollbar should be contained by viewport"
        );
        // scrollbar should fill viewport if too small for SCROLLBAR_MIN_SIZE
        assert_approx_eq!(
            f64,
            scrollbar_rect.y0,
            viewport.view_rect().y0 + TEST_SCROLLBAR_PAD
        );
        assert_approx_eq!(
            f64,
            scrollbar_rect.y1,
            viewport.view_rect().y1 - TEST_SCROLLBAR_PAD
        );
    }

    #[test]
    fn scrollbar_layout_viewport_too_small_for_bar() {
        let mut scroll_component = ScrollComponent::new();
        scroll_component.enabled = ScrollbarsEnabled::Vertical;
        let viewport = Viewport {
            content_size: Size::new(100.0, 100.0),
            view_origin: (0.0, 25.0).into(),
            view_size: (100.0, 3.0).into(),
        };

        let scrollbar_rect = scroll_component.calc_vertical_bar_bounds(&viewport, &test_env());

        assert_eq!(
            scrollbar_rect, None,
            "scrollbar should not be drawn if viewport is too small"
        );
    }

    fn rect_contains(outer: Rect, inner: Rect) -> bool {
        outer.union(inner) == outer
    }

    fn test_env() -> Env {
        Env::empty()
            .adding(theme::SCROLLBAR_WIDTH, TEST_SCROLLBAR_WIDTH)
            .adding(theme::SCROLLBAR_PAD, TEST_SCROLLBAR_PAD)
            .adding(theme::SCROLLBAR_MIN_SIZE, TEST_SCROLLBAR_MIN_SIZE)
    }
}
