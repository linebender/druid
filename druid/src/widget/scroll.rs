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

use crate::kurbo::{Affine, Point, Rect, Size, Vec2};
use crate::theme;
use crate::{
    scroll_component::*, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, TimerToken, UpdateCtx, Widget, WidgetPod,
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
        let size = ctx.size();
        let viewport = Rect::from_origin_size(Point::ORIGIN, size);

        let scrollbar_is_hovered = match event {
            Event::MouseMove(e) | Event::MouseUp(e) | Event::MouseDown(e) => {
                let offset_pos = e.pos + self.scroll_component.scroll_offset;
                self.scroll_component
                    .point_hits_vertical_bar(viewport, offset_pos, env)
                    || self
                        .scroll_component
                        .point_hits_horizontal_bar(viewport, offset_pos, env)
            }
            _ => false,
        };

        if self.scroll_component.scrollbars.are_held() {
            // if we're dragging a scrollbar
            match event {
                Event::MouseMove(event) => {
                    match self.scroll_component.scrollbars.held {
                        BarHeldState::Vertical(offset) => {
                            let scale_y =
                                viewport.height() / self.scroll_component.content_size.height;
                            let bounds = self
                                .scroll_component
                                .calc_vertical_bar_bounds(viewport, env);
                            let mouse_y = event.pos.y + self.scroll_component.scroll_offset.y;
                            let delta = mouse_y - bounds.y0 - offset;
                            self.scroll_component
                                .scroll(Vec2::new(0f64, (delta / scale_y).ceil()), size);
                        }
                        BarHeldState::Horizontal(offset) => {
                            let scale_x =
                                viewport.width() / self.scroll_component.content_size.width;
                            let bounds = self
                                .scroll_component
                                .calc_horizontal_bar_bounds(viewport, env);
                            let mouse_x = event.pos.x + self.scroll_component.scroll_offset.x;
                            let delta = mouse_x - bounds.x0 - offset;
                            self.scroll_component
                                .scroll(Vec2::new((delta / scale_x).ceil(), 0f64), size);
                        }
                        _ => (),
                    }
                    ctx.request_paint();
                }
                Event::MouseUp(_) => {
                    self.scroll_component.scrollbars.held = BarHeldState::None;
                    ctx.set_active(false);

                    if !scrollbar_is_hovered {
                        self.scroll_component.scrollbars.hovered = BarHoveredState::None;
                        self.scroll_component
                            .reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                    }
                }
                _ => (), // other events are a noop
            }
        } else if scrollbar_is_hovered {
            // if we're over a scrollbar but not dragging
            match event {
                Event::MouseMove(event) => {
                    let offset_pos = event.pos + self.scroll_component.scroll_offset;
                    if self
                        .scroll_component
                        .point_hits_vertical_bar(viewport, offset_pos, env)
                    {
                        self.scroll_component.scrollbars.hovered = BarHoveredState::Vertical;
                    } else {
                        self.scroll_component.scrollbars.hovered = BarHoveredState::Horizontal;
                    }

                    self.scroll_component.scrollbars.opacity =
                        env.get(theme::SCROLLBAR_MAX_OPACITY);
                    self.scroll_component.scrollbars.timer_id = TimerToken::INVALID; // Cancel any fade out in progress
                    ctx.request_paint();
                }
                Event::MouseDown(event) => {
                    let pos = event.pos + self.scroll_component.scroll_offset;

                    if self
                        .scroll_component
                        .point_hits_vertical_bar(viewport, pos, env)
                    {
                        ctx.set_active(true);
                        self.scroll_component.scrollbars.held = BarHeldState::Vertical(
                            pos.y
                                - self
                                    .scroll_component
                                    .calc_vertical_bar_bounds(viewport, env)
                                    .y0,
                        );
                    } else if self
                        .scroll_component
                        .point_hits_horizontal_bar(viewport, pos, env)
                    {
                        ctx.set_active(true);
                        self.scroll_component.scrollbars.held = BarHeldState::Horizontal(
                            pos.x
                                - self
                                    .scroll_component
                                    .calc_horizontal_bar_bounds(viewport, env)
                                    .x0,
                        );
                    }
                }
                // if the mouse was downed elsewhere, moved over a scroll bar and released: noop.
                Event::MouseUp(_) => (),
                _ => unreachable!(),
            }
        } else {
            let force_event = self.child.is_hot() || self.child.is_active();
            let child_event =
                event.transform_scroll(self.scroll_component.scroll_offset, viewport, force_event);
            if let Some(child_event) = child_event {
                self.child.event(ctx, &child_event, data, env);
            };

            match event {
                Event::MouseMove(_) => {
                    // if we have just stopped hovering
                    if self.scroll_component.scrollbars.hovered.is_hovered()
                        && !scrollbar_is_hovered
                    {
                        self.scroll_component.scrollbars.hovered = BarHoveredState::None;
                        self.scroll_component
                            .reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                    }
                }
                Event::Timer(id) if *id == self.scroll_component.scrollbars.timer_id => {
                    // Schedule scroll bars animation
                    ctx.request_anim_frame();
                    self.scroll_component.scrollbars.timer_id = TimerToken::INVALID;
                }
                _ => (),
            }
        }

        if !ctx.is_handled() {
            if let Event::Wheel(mouse) = event {
                if self.scroll_component.scroll(mouse.wheel_delta, size) {
                    ctx.request_paint();
                    ctx.set_handled();
                    self.scroll_component
                        .reset_scrollbar_fade(|d| ctx.request_timer(d), env);
                }
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::AnimFrame(interval) => {
                // Guard by the timer id being invalid, otherwise the scroll bars would fade
                // immediately if some other widgeet started animating.
                if self.scroll_component.scrollbars.timer_id == TimerToken::INVALID {
                    // Animate scroll bars opacity
                    let diff = 2.0 * (*interval as f64) * 1e-9;
                    self.scroll_component.scrollbars.opacity -= diff;
                    if self.scroll_component.scrollbars.opacity > 0.0 {
                        ctx.request_anim_frame();
                    }
                }
            }
            // Show the scrollbars any time our size changes
            LifeCycle::Size(_) => self
                .scroll_component
                .reset_scrollbar_fade(|d| ctx.request_timer(d), &env),
            _ => (),
        }
        self.child.lifecycle(ctx, event, data, env)
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
        let viewport = ctx.size().to_rect();
        ctx.with_save(|ctx| {
            ctx.clip(viewport);
            ctx.transform(Affine::translate(-self.scroll_component.scroll_offset));

            let visible = ctx.region().to_rect() + self.scroll_component.scroll_offset;
            ctx.with_child_ctx(visible, |ctx| self.child.paint_raw(ctx, data, env));

            self.scroll_component.draw_bars(ctx, viewport, env);
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
