use std::mem;

use crate::kurbo::{Rect, RoundedRect, Size};
use crate::theme;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, RenderContext, Selector,
    UpdateCtx, Widget,
};
use std::cell::RefCell;
use std::marker::PhantomData;

/// Trait used for shared state between
/// the scrollbar and the scroll container.
pub trait ScrollControlState: Data + PartialEq {
    /// The last position reported by a MouseMove event.
    /// used to calculate the deltas without maintaining
    /// an offset.
    fn last_mouse_pos(&self) -> f64;
    /// The identifier for this state object
    fn id(&self) -> u64;
    /// The size of a page. Used to calculate
    /// scroll distances when clicking on the
    /// scroll track
    fn page_size(&self) -> f64;
    /// The maximum distance in pixels to scroll
    fn max_scroll_position(&self) -> f64;
    /// The minimum distance in pixels to scroll
    fn min_scroll_position(&self) -> f64;
    /// boolean for enabling/disabling the mouse
    fn mouse_wheel_enabled(&self) -> bool;
    /// scrollbar travel distance / scroll container travel distance
    fn scale(&self) -> f64;
    /// The current scroll position. This value
    /// is always between min and max scroll positions
    fn scroll_position(&self) -> f64;
    /// true when tracking mouse move events
    fn tracking_mouse(&self) -> bool;

    fn set_last_mouse_pos(&mut self, val: f64);
    fn set_page_size(&mut self, val: f64);
    fn set_max_scroll_position(&mut self, val: f64);
    fn set_min_scroll_position(&mut self, val: f64);
    fn set_mouse_wheel_enabled(&mut self, val: bool);
    fn set_tracking_mouse(&mut self, val: bool);
    fn set_scale(&mut self, val: f64);
    /// Sets the raw value for the scroll position
    /// without respecting min and max. Use this
    /// for pull or bounce calculations on the container
    fn set_scroll_position(&mut self, val: f64);

    fn set_scroll_pos_from_delta(&mut self, delta: f64) -> f64 {
        let scroll_position = self.scroll_position() + delta;
        let clamped_scroll_position = self
            .min_scroll_position()
            .max(scroll_position.min(self.max_scroll_position()));
        self.set_scroll_position(clamped_scroll_position);

        clamped_scroll_position
    }
}

struct AnimationState {
    elapsed_time: u64,
    pub current_value: f64,
    pub delay: u64,
    pub done: bool,
    pub duration: u64,
    pub equation: fn(t: f64, b: f64, c: f64, d: f64) -> f64,
    pub from: f64,
    pub to: f64,
}

/// A time-based animation struct
impl AnimationState {
    /// Increments the animation based on the
    /// specified interval.
    pub fn tick(&mut self, interval: u64) -> f64 {
        self.elapsed_time += interval;
        if interval == 0 || self.elapsed_time < self.delay {
            return self.current_value;
        }
        let t = (self.elapsed_time - self.delay) as f64 / self.duration as f64;
        // Progress as a percentage between 0 and 1
        let mut progress = (self.equation)(t, 0., 1., 1.);
        // We're done if we've hit 1.0 or greater.
        if progress >= 1. {
            progress = 1.;
            self.done = true;
        }
        self.current_value = self.from + ((self.to - self.from) * progress);
        self.current_value
    }

    pub fn reset(&mut self) {
        self.current_value = self.from;
        self.elapsed_time = 0;
        self.done = false;
    }

    pub fn reverse(&mut self) {
        let from = mem::replace(&mut self.from, self.to);
        self.to = from;
        // Inverse the elapsed time for reversals
        // that happen during an ongoing animation
        // this prevents the animation from taking
        // the entire duration when it only has a
        // portion of the interpolation left to go.
        if self.done {
            self.reset();
        } else if self.elapsed_time > 0 {
            let total_time = self.duration + self.delay;
            self.elapsed_time = total_time * (1 - (self.elapsed_time / total_time));
        }
        self.done = self.to.same(&self.current_value);
    }
}

impl Default for AnimationState {
    fn default() -> Self {
        AnimationState {
            current_value: 0.,
            elapsed_time: 0,
            delay: 0,
            done: false,
            duration: 500_000_000,
            from: 0.,
            to: 1.,
            equation: |t: f64, b: f64, c: f64, d: f64| -> f64 {
                // cubic ease out
                let t = t / d - 1.;
                c * (t * t * t + 1.) + b
            },
        }
    }
}

pub struct Scrollbar<S: ScrollControlState> {
    animation_state: AnimationState,
    opacity: f64,
    is_hot: bool,
    scroll_policy: ScrollPolicy,
    state: PhantomData<S>,
}

impl<S: ScrollControlState> Scrollbar<S> {
    pub fn new() -> Scrollbar<S> {
        Scrollbar {
            animation_state: Default::default(),
            is_hot: false,
            opacity: 0.,
            scroll_policy: ScrollPolicy::Auto,

            state: Default::default(),
        }
    }

    pub fn scroll_policy(mut self, val: ScrollPolicy) -> Self {
        self.scroll_policy = val;
        self
    }

    fn calculated_thumb_size(&self, state: &S, env: &Env, size: &Size) -> f64 {
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);
        let min_scroll_position = state.min_scroll_position();
        let max_scroll_position = state.max_scroll_position();
        let page_size = state.page_size();

        let extent = size.height.max(size.width);
        let target_thumb_size =
            (page_size / (max_scroll_position - min_scroll_position + page_size)) * extent;
        target_thumb_size.max(bar_width * 2.)
    }

    fn calculated_thumb_rect(&self, state: &S, env: &Env, size: &Size) -> Rect {
        let min_scroll_position = state.min_scroll_position();
        let max_scroll_position = state.max_scroll_position();
        let thumb_size = self.calculated_thumb_size(state, env, &size);
        let distance = size.height.max(size.width);
        let scale = (distance - thumb_size) / (max_scroll_position - min_scroll_position);
        let scaled_scroll_position = state.scroll_position() * scale;
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);

        if size.width > size.height {
            // Horizontal
            Rect::new(
                scaled_scroll_position,
                0.,
                scaled_scroll_position + thumb_size,
                bar_width,
            )
        } else {
            // Vertical
            Rect::new(
                0.,
                scaled_scroll_position,
                bar_width,
                scaled_scroll_position + thumb_size,
            )
        }
    }

    fn show(&mut self) {
        // Animation is already fading in
        if self.animation_state.to.same(&1.) {
            return;
        }
        if self.animation_state.to.same(&0.) {
            self.animation_state.reverse();
        }
        self.animation_state.delay = 0;
    }

    fn hide(&mut self, env: &Env) {
        // Animation is already fading out
        if self.animation_state.to.same(&0.) {
            return;
        }

        if self.animation_state.to.same(&1.) {
            self.animation_state.reverse();
        }
        self.animation_state.delay = env.get(theme::SCROLL_BAR_FADE_DELAY) * 1_000_000;
    }
}

impl<S: ScrollControlState> Default for Scrollbar<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: ScrollControlState> Widget<RefCell<S>> for Scrollbar<S> {
    fn event(&mut self, event_ctx: &mut EventCtx, event: &Event, data: &mut RefCell<S>, env: &Env) {
        if self.scroll_policy == ScrollPolicy::Off {
            return;
        }
        let size = event_ctx.size();
        match event {
            Event::AnimFrame(interval) => {
                if !self.animation_state.done {
                    self.opacity = self.animation_state.tick(*interval);
                    event_ctx.request_anim_frame();
                } else if !self.is_hot && self.opacity > 0. {
                    self.hide(env);
                    event_ctx.request_anim_frame();
                }
            }

            Event::Command(command) => {
                let Selector(id) = command.selector;
                if id == "scroll" && *command.get_object().unwrap_or(&0) == data.borrow().id() {
                    self.show();
                    event_ctx.request_anim_frame();
                }
            }

            Event::HotChanged(is_hot) => {
                self.is_hot = *is_hot;
                if self.is_hot {
                    self.show();
                } else {
                    self.hide(env);
                    event_ctx.request_anim_frame();
                }
            }

            Event::MouseDown(event) => {
                // Do nothing if we're hidden
                if self.opacity == 0. {
                    return;
                }
                let state = data.get_mut();
                // Set our scale since we could be dragging later
                // The thumb size is subtracted from the total
                // scrollable distance and a scale is calculated
                // to translate scrollbar distance to scroll container distance
                let distance = size.width.max(size.height);
                let thumb_size = self.calculated_thumb_size(state, env, &size);
                let scale = (distance - thumb_size)
                    / (state.max_scroll_position() - state.min_scroll_position());
                state.set_scale(scale);

                // Determine if we're over the thumb.
                // If so, prepare it for dragging,
                // if not, page the scroll_position.
                let hit_test_rect = self.calculated_thumb_rect(state, env, &size);
                if hit_test_rect.contains(event.pos) {
                    state.set_tracking_mouse(true);
                    state.set_last_mouse_pos(if size.width > size.height {
                        event.pos.x
                    } else {
                        event.pos.y
                    });
                } else {
                    let center = hit_test_rect.center();
                    let delta = if center.x > event.pos.x || center.y > event.pos.y {
                        -state.page_size()
                    } else {
                        state.page_size()
                    };
                    state.set_scroll_pos_from_delta(delta);
                    event_ctx.invalidate();
                }
            }

            Event::MouseMoved(event) => {
                let state = data.get_mut();
                if !state.tracking_mouse() {
                    return;
                }
                let pos = if size.width > size.height {
                    event.pos.x
                } else {
                    event.pos.y
                };
                let delta = pos - state.last_mouse_pos();
                let scale = state.scale();
                state.set_scroll_pos_from_delta(delta / scale);
                state.set_last_mouse_pos(pos);
                event_ctx.invalidate();
            }

            Event::MouseUp(_) => {
                let state = data.get_mut();
                state.set_tracking_mouse(false);
                state.set_last_mouse_pos(0.);
            }

            Event::Size(_) => {
                self.show();
                event_ctx.request_anim_frame();
            }

            Event::Wheel(event) => {
                let state = data.get_mut();
                if !state.mouse_wheel_enabled() {
                    return;
                }
                let delta = if size.width > size.height {
                    event.delta.x
                } else {
                    event.delta.y
                };
                self.show();
                state.set_scroll_pos_from_delta(delta);
                event_ctx.request_anim_frame();
            }

            _ => (),
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: Option<&RefCell<S>>,
        data: &RefCell<S>,
        _env: &Env,
    ) {
        if let Some(old) = old_data {
            let old_state = old.borrow();
            let state = data.borrow();
            if old_state.max_scroll_position() - state.max_scroll_position() != 0.
                || old_state.min_scroll_position() - state.min_scroll_position() != 0.
                || old_state.scroll_position() - state.scroll_position() != 0.
            {
                ctx.invalidate();
            }
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &RefCell<S>,
        _env: &Env,
    ) -> Size {
        bc.constrain(bc.max())
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &RefCell<S>, env: &Env) {
        let state = data.borrow();
        if state.max_scroll_position() < 0. {
            return;
        }
        let brush = paint_ctx
            .render_ctx
            .solid_brush(env.get(theme::SCROLL_BAR_COLOR).with_alpha(self.opacity));
        let border_brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_BORDER_COLOR)
                .with_alpha(self.opacity),
        );

        let radius = env.get(theme::SCROLL_BAR_RADIUS);
        let edge_width = env.get(theme::SCROLL_BAR_EDGE_WIDTH);

        let size = paint_ctx.size();
        let bounds = self.calculated_thumb_rect(&state, env, &size);
        let rect = RoundedRect::from_rect(bounds, radius);
        paint_ctx.render_ctx.fill(rect, &brush);
        paint_ctx.render_ctx.stroke(rect, &border_brush, edge_width);
    }
}

#[derive(PartialEq)]
pub enum ScrollPolicy {
    On,
    Off,
    Auto,
}
