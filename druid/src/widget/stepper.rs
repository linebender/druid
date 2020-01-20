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

//! A stepper widget.

use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    TimerToken, UpdateCtx, Widget,
};
use std::f64::EPSILON;
use std::time::{Duration, Instant};

use crate::kurbo::{BezPath, Rect, RoundedRect};
use crate::piet::{LinearGradient, RenderContext, UnitPoint};

use crate::theme;
use crate::Point;

// Delay until stepper starts automatically changing valued when one of the button is held down.
const STEPPER_REPEAT_DELAY: Duration = Duration::from_millis(500);
// Delay between value changes when one of the button is held down.
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
    pub fn new() -> Self {
        Stepper {
            max: std::f64::MAX,
            min: std::f64::MIN,
            step: 1.0,
            wrap: false,
            increase_active: false,
            decrease_active: false,
            timer_id: TimerToken::INVALID,
        }
    }

    /// Set the stepper's maximum value.
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Set the stepper's minimum value.
    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Set the steppers amount by which the value increases or decreases.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set whether the stepper should wrap around the minimum/maximum values.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    fn change_value(&mut self, _ctx: &mut EventCtx, data: &mut f64, _env: &Env) {
        // increase/decrease value depending on which button is currently active
        let delta = if self.increase_active {
            self.step
        } else if self.decrease_active {
            -1. * self.step
        } else {
            0.0
        };

        *data = (*data + delta).max(self.min).min(self.max);

        if self.wrap {
            if (*data - self.min).abs() < EPSILON {
                *data = self.max
            } else {
                *data = self.min
            }
        }
    }
}

impl Default for Stepper {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget<f64> for Stepper {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _data: &f64, env: &Env) {
        let rounded_rect =
            RoundedRect::from_origin_size(Point::ORIGIN, paint_ctx.size().to_vec2(), 4.);

        let height = paint_ctx.size().height;
        let width = env.get(theme::BASIC_WIDGET_HEIGHT);
        let button_size = Size::new(width, height / 2.);

        paint_ctx.stroke(rounded_rect, &env.get(theme::BORDER), 2.0);
        paint_ctx.clip(rounded_rect);

        // draw buttons for increase/decrease
        let increase_button_origin = Point::ORIGIN;
        let mut decrease_button_origin = Point::ORIGIN;
        decrease_button_origin.y += height / 2.;

        let increase_button_rect = Rect::from_origin_size(increase_button_origin, button_size);
        let decrease_button_rect = Rect::from_origin_size(decrease_button_origin, button_size);

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
        if self.increase_active {
            paint_ctx.fill(increase_button_rect, &active_gradient);
        } else {
            paint_ctx.fill(increase_button_rect, &inactive_gradient);
        };

        if self.decrease_active {
            paint_ctx.fill(decrease_button_rect, &active_gradient);
        } else {
            paint_ctx.fill(decrease_button_rect, &inactive_gradient);
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

        paint_ctx.fill(arrows, &env.get(theme::LABEL_COLOR));
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        env: &Env,
    ) -> Size {
        bc.constrain(Size::new(
            env.get(theme::BASIC_WIDGET_HEIGHT),
            env.get(theme::BORDERED_WIDGET_HEIGHT),
        ))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);

        match event {
            Event::MouseDown(mouse) => {
                ctx.set_active(true);

                if mouse.pos.y > height / 2. {
                    self.decrease_active = true;
                } else {
                    self.increase_active = true;
                }

                self.change_value(ctx, data, env);

                let delay = Instant::now() + STEPPER_REPEAT_DELAY;
                self.timer_id = ctx.request_timer(delay);

                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                ctx.set_active(false);

                self.decrease_active = false;
                self.increase_active = false;
                self.timer_id = TimerToken::INVALID;

                ctx.invalidate();
            }
            Event::Timer(id) if *id == self.timer_id => {
                self.change_value(ctx, data, env);
                let delay = Instant::now() + STEPPER_REPEAT;
                self.timer_id = ctx.request_timer(delay);
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &f64, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &f64, data: &f64, _env: &Env) {
        if (*data - old_data).abs() > EPSILON {
            ctx.invalidate();
        }
    }
}
