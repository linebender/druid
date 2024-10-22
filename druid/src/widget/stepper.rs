// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A stepper widget.

use std::time::Duration;
use tracing::{instrument, trace};

use crate::debug_state::DebugState;
use crate::kurbo::BezPath;
use crate::piet::{LinearGradient, RenderContext, UnitPoint};
use crate::widget::prelude::*;
use crate::{theme, Point, Rect, TimerToken};

// Delay until stepper starts automatically changing value when one of the buttons is held down.
const STEPPER_REPEAT_DELAY: Duration = Duration::from_millis(500);
// Delay between value changes when one of the buttons is held down.
const STEPPER_REPEAT: Duration = Duration::from_millis(200);

/// A stepper widget for step-wise increasing and decreasing a value.
pub struct Stepper {
    max: f64,
    min: f64,
    step: f64,
    wrap: bool,
    /// Keeps track of which button is currently triggered.
    increase_active: bool,
    decrease_active: bool,
    timer_id: TimerToken,
}

impl Stepper {
    /// Create a new `Stepper`.
    pub fn new() -> Self {
        Stepper {
            max: f64::MAX,
            min: f64::MIN,
            step: 1.0,
            wrap: false,
            increase_active: false,
            decrease_active: false,
            timer_id: TimerToken::INVALID,
        }
    }

    /// Set the range covered by this slider.
    ///
    /// The default range is `std::f64::MIN..std::f64::MAX`.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set the steppers amount by which the value increases or decreases.
    ///
    /// The default step is `1.0`.
    pub fn with_step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set whether the stepper should wrap around the minimum/maximum values.
    ///
    /// When wraparound is enabled incrementing above max behaves like this:
    /// - if the previous value is < max it becomes max
    /// - if the previous value is = max it becomes min
    ///
    /// Same logic applies for decrementing.
    ///
    /// The default is `false`.
    pub fn with_wraparound(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    fn increment(&mut self, data: &mut f64) {
        let next = *data + self.step;
        let was_greater = *data + f64::EPSILON >= self.max;
        let is_greater = next + f64::EPSILON > self.max;
        *data = match (self.wrap, was_greater, is_greater) {
            (true, true, true) => self.min,
            (true, false, true) => self.max,
            (false, _, true) => self.max,
            _ => next,
        }
    }

    fn decrement(&mut self, data: &mut f64) {
        let next = *data - self.step;
        let was_less = *data - f64::EPSILON <= self.min;
        let is_less = next - f64::EPSILON < self.min;
        *data = match (self.wrap, was_less, is_less) {
            (true, true, true) => self.max,
            (true, false, true) => self.min,
            (false, _, true) => self.min,
            _ => next,
        }
    }
}

impl Default for Stepper {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget<f64> for Stepper {
    #[instrument(name = "Stepper", level = "trace", skip(self, ctx, _data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &f64, env: &Env) {
        let stroke_width = 2.0;
        let rounded_rect = ctx
            .size()
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(4.0);

        let height = ctx.size().height;
        let width = env.get(theme::BASIC_WIDGET_HEIGHT);
        let button_size = Size::new(width, height / 2.);

        ctx.stroke(rounded_rect, &env.get(theme::BORDER_DARK), stroke_width);
        ctx.clip(rounded_rect);

        // draw buttons for increase/decrease
        let increase_button_origin = Point::ORIGIN;
        let decrease_button_origin = Point::new(0., height / 2.0);

        let increase_button_rect = Rect::from_origin_size(increase_button_origin, button_size);
        let decrease_button_rect = Rect::from_origin_size(decrease_button_origin, button_size);

        let disabled_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::DISABLED_BUTTON_LIGHT),
                env.get(theme::DISABLED_BUTTON_DARK),
            ),
        );

        let active_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (env.get(theme::PRIMARY_LIGHT), env.get(theme::PRIMARY_DARK)),
        );

        let inactive_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (env.get(theme::BUTTON_DARK), env.get(theme::BUTTON_LIGHT)),
        );

        // draw buttons that are currently triggered as active
        if ctx.is_disabled() {
            ctx.fill(increase_button_rect, &disabled_gradient);
        } else if self.increase_active {
            ctx.fill(increase_button_rect, &active_gradient);
        } else {
            ctx.fill(increase_button_rect, &inactive_gradient);
        };

        if ctx.is_disabled() {
            ctx.fill(decrease_button_rect, &disabled_gradient);
        } else if self.decrease_active {
            ctx.fill(decrease_button_rect, &active_gradient);
        } else {
            ctx.fill(decrease_button_rect, &inactive_gradient);
        };

        // draw up and down triangles
        let mut arrows = BezPath::new();
        arrows.move_to(Point::new(4., height / 2. - 4.));
        arrows.line_to(Point::new(width - 4., height / 2. - 4.));
        arrows.line_to(Point::new(width / 2., 4.));
        arrows.close_path();

        arrows.move_to(Point::new(4., height / 2. + 4.));
        arrows.line_to(Point::new(width - 4., height / 2. + 4.));
        arrows.line_to(Point::new(width / 2., height - 4.));
        arrows.close_path();

        let color = if ctx.is_disabled() {
            env.get(theme::DISABLED_TEXT_COLOR)
        } else {
            env.get(theme::TEXT_COLOR)
        };

        ctx.fill(arrows, &color);
    }

    #[instrument(
        name = "Stepper",
        level = "trace",
        skip(self, _layout_ctx, bc, _data, env)
    )]
    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        env: &Env,
    ) -> Size {
        let size = bc.constrain(Size::new(
            env.get(theme::BASIC_WIDGET_HEIGHT),
            env.get(theme::BORDERED_WIDGET_HEIGHT),
        ));
        trace!("Computed size: {}", size);
        size
    }

    #[instrument(name = "Stepper", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);

        match event {
            Event::MouseDown(mouse) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);

                    if mouse.pos.y > height / 2. {
                        self.decrease_active = true;
                        self.decrement(data);
                    } else {
                        self.increase_active = true;
                        self.increment(data);
                    }

                    self.timer_id = ctx.request_timer(STEPPER_REPEAT_DELAY);

                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                ctx.set_active(false);

                self.decrease_active = false;
                self.increase_active = false;
                self.timer_id = TimerToken::INVALID;

                ctx.request_paint();
            }
            Event::Timer(id) if *id == self.timer_id => {
                if !ctx.is_disabled() {
                    if self.increase_active {
                        self.increment(data);
                    }
                    if self.decrease_active {
                        self.decrement(data);
                    }
                    self.timer_id = ctx.request_timer(STEPPER_REPEAT);
                } else {
                    ctx.set_active(false);
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &f64, _env: &Env) {
        if let LifeCycle::DisabledChanged(_) = event {
            ctx.request_paint();
        }
    }

    #[instrument(
        name = "Stepper",
        level = "trace",
        skip(self, ctx, old_data, data, _env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &f64, data: &f64, _env: &Env) {
        if (*data - old_data).abs() > f64::EPSILON {
            ctx.request_paint();
        }
    }

    fn debug_state(&self, data: &f64) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: data.to_string(),
            ..Default::default()
        }
    }
}
