use std::mem;

use crate::{BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, RenderContext, Selector, UpdateCtx, Widget};
use crate::kurbo::{Rect, RoundedRect, Size};
use crate::theme;

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

    fn set_scroll_pos_from_delta(&mut self, delta: f64) {
        let scroll_position = self.scroll_position() + delta;

        self.set_scroll_position(self.min_scroll_position()
            .max(scroll_position
                .min(self.max_scroll_position())));
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
            duration: 250_000_000,
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

pub struct Scrollbar {
    animation_state: AnimationState,
    opacity: f64,
    is_hot: bool,
    scroll_policy: ScrollPolicy,
}

impl Scrollbar {
    pub fn scroll_policy(mut self, val: ScrollPolicy) -> Self {
        self.scroll_policy = val;
        self
    }

    fn calculated_thumb_size(data: &impl ScrollControlState, env: &Env, size: &Size) -> f64 {
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);
        let min_scroll_position = data.min_scroll_position();
        let max_scroll_position = data.max_scroll_position();
        let page_size = data.page_size();

        let extent = size.height.max(size.width);
        let target_thumb_size = (page_size / (max_scroll_position - min_scroll_position + page_size)) * extent;
        target_thumb_size.max(bar_width * 2.)
    }

    fn calculated_thumb_rect(data: &impl ScrollControlState, env: &Env, size: &Size) -> Rect {
        let min_scroll_position = data.min_scroll_position();
        let max_scroll_position = data.max_scroll_position();
        let thumb_size = Scrollbar::calculated_thumb_size(data, env, &size);
        let distance = size.height.max(size.width);
        let scale = (distance - thumb_size) / (max_scroll_position - min_scroll_position);
        let scaled_scroll_position = data.scroll_position() * scale;
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);

        if size.width > size.height {
            // Horizontal
            Rect::new(scaled_scroll_position, 0., scaled_scroll_position + thumb_size, bar_width * 2.)
        } else {
            // Vertical
            Rect::new(0., scaled_scroll_position, bar_width, scaled_scroll_position + thumb_size)
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

impl Default for Scrollbar {
    fn default() -> Self {
        Scrollbar {
            animation_state: Default::default(),
            is_hot: false,
            opacity: 0.,
            scroll_policy: ScrollPolicy::Auto,
        }
    }
}

impl<T: ScrollControlState> Widget<T> for Scrollbar {
    fn event(&mut self, event_ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let size = event_ctx.size();
        match event {
            Event::Command(command) => {
                let Selector(id) = command.selector;
                if id == "scroll"
                    && *command.get_object().unwrap_or(&0) == data.id() {
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

            Event::AnimFrame(interval) => {
                if !self.animation_state.done {
                    self.opacity = self.animation_state.tick(*interval);
                    event_ctx.request_anim_frame();
                    event_ctx.invalidate();
                } else if !self.is_hot && self.opacity > 0. {
                    self.hide(env);
                    event_ctx.request_anim_frame();
                }
            }

            Event::Wheel(event) => {
                if !data.mouse_wheel_enabled() {
                    return;
                }
                let delta = if size.width > size.height { event.delta.x } else { event.delta.y };
                self.show();
                data.set_scroll_pos_from_delta(delta);
                event_ctx.invalidate();
                event_ctx.request_anim_frame();
            }

            Event::MouseMoved(event) => {
                if !data.tracking_mouse() {
                    return;
                }
                let pos = if size.width > size.height { event.pos.x } else { event.pos.y };
                let delta = pos - data.last_mouse_pos();

                data.set_scroll_pos_from_delta(delta / data.scale());
                data.set_last_mouse_pos(pos);
                event_ctx.invalidate();
            }

            Event::MouseDown(event) => {
                // Do nothing if we're hidden
                if self.opacity == 0. {
                    return;
                }
                // Set our scale since we could be dragging later
                // The thumb size is subtracted from the total
                // scrollable distance and a scale is calculated
                // to translate scrollbar distance to scroll container distance
                let distance = size.width.max(size.height);
                let thumb_size = Scrollbar::calculated_thumb_size(data, env, &size);
                let scale = (distance - thumb_size) / (data.max_scroll_position() - data.min_scroll_position());
                data.set_scale(scale);

                // Determine if we're over the thumb.
                // If so, prepare it for dragging,
                // if not, page the scroll_position.
                let hit_test_rect = Scrollbar::calculated_thumb_rect(data, env, &size);
                if hit_test_rect.contains(event.pos) {
                    data.set_tracking_mouse(true);
                    data.set_last_mouse_pos(if size.width > size.height { event.pos.x } else { event.pos.y });
                } else {
                    let center = hit_test_rect.center();
                    let delta = if center.x > event.pos.x || center.y > event.pos.y {
                        -data.page_size()
                    } else {
                        data.page_size()
                    };
                    data.set_scroll_pos_from_delta(delta);
                    event_ctx.invalidate();
                }
            }

            Event::MouseUp(_) => {
                data.set_tracking_mouse(false);
                data.set_last_mouse_pos(0.);
            }

            _ => ()
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, _env: &Env) {
        if let Some(old) = old_data {
            if old.max_scroll_position() - data.max_scroll_position() != 0.
                || old.min_scroll_position() - data.min_scroll_position() != 0.
                || old.scroll_position() - data.scroll_position() != 0. {
                ctx.invalidate();
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.constrain(bc.max())
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        if data.max_scroll_position() < 0. {
            return;
        }
        let brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_COLOR)
                .with_alpha(self.opacity),
        );
        let border_brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_BORDER_COLOR)
                .with_alpha(self.opacity),
        );

        let radius = env.get(theme::SCROLL_BAR_RADIUS);
        let edge_width = env.get(theme::SCROLL_BAR_EDGE_WIDTH);

        let size = paint_ctx.size();
        let bounds = Scrollbar::calculated_thumb_rect(data, env, &size);
        let rect = RoundedRect::from_rect(bounds, radius);
        paint_ctx.render_ctx.fill(rect, &brush);
        paint_ctx.render_ctx.stroke(rect, &border_brush, edge_width);
    }
}

pub enum ScrollPolicy {
    On,
    Off,
    Auto,
}

#[cfg(test)]
mod scrollbar_tests {
    use crate::widget::scrollbar::AnimationState;

    #[test]
    fn animation_state_test() {
        let mut state = AnimationState::default();
        let val = state.tick(1);
        assert_eq!(val, 23.);
    }
}