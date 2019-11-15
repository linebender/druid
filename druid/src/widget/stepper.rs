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
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, TimerToken,
    UpdateCtx, Widget,
};
use std::time::{Duration, Instant};

use crate::kurbo::{BezPath, Rect, RoundedRect};
use crate::piet::{
    FontBuilder, LinearGradient, RenderContext, Text, TextLayout, TextLayoutBuilder, UnitPoint,
};

use crate::theme;
use crate::widget::{Align, Label, LabelText, SizedBox};
use crate::Point;

/// A stepper.
pub struct Stepper {
    max: f64,
    min: f64,
    step: f64,
    wrap: bool,
    /// A closure that will be invoked when the value changed.
    value_changed: Box<dyn Fn(&mut EventCtx, &mut f64, &Env)>,
    /// Keeps track of which button is currently triggered.
    increase_active: bool,
    decrease_active: bool,
    timer_id: TimerToken,
}

impl Stepper {
    pub fn new(
        max: f64,
        min: f64,
        step: f64,
        wrap: bool,
        value_changed: impl Fn(&mut EventCtx, &mut f64, &Env) + 'static,
    ) -> impl Widget<f64> {
        Align::vertical(
            UnitPoint::CENTER,
            Stepper {
                max,
                min,
                step,
                wrap,
                value_changed: Box::new(value_changed),
                increase_active: false,
                decrease_active: false,
                timer_id: TimerToken::INVALID,
            },
        )
    }

    fn change_value(&mut self, ctx: &mut EventCtx, data: &mut f64, env: &Env) {
        let delta = if self.increase_active {
            self.step
        } else if self.decrease_active {
            -1. * self.step
        } else {
            0.0
        };

        let old_data = *data;
        *data = (*data + delta).min(self.min).max(self.max);

        if old_data != *data {
            (self.value_changed)(ctx, data, env);
        } else {
            if self.wrap {
                if *data == self.min {
                    *data = self.max
                } else {
                    *data = self.min
                }
            }
        }
    }
}

impl Widget<f64> for Stepper {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &f64, env: &Env) {
        let rounded_rect =
            RoundedRect::from_origin_size(Point::ORIGIN, base_state.size().to_vec2(), 4.);

        let height = base_state.size().height;
        let width = env.get(theme::BASIC_WIDGET_HEIGHT);
        let button_size = Size::new(width, height / 2.);

        paint_ctx.stroke(rounded_rect, &env.get(theme::BORDER), 2.0);
        paint_ctx.clip(rounded_rect);

        // draw buttons for increase/decrease
        let mut increase_button_origin = Point::ORIGIN;
        let mut decrease_button_origin = Point::ORIGIN;
        decrease_button_origin.y += height / 2.;

        let increase_rect = Rect::from_origin_size(increase_button_origin, button_size);
        let decrease_rect = Rect::from_origin_size(decrease_button_origin, button_size);

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
            paint_ctx.fill(increase_rect, &active_gradient);
        } else {
            paint_ctx.fill(increase_rect, &inactive_gradient);
        };

        if self.decrease_active {
            paint_ctx.fill(decrease_rect, &active_gradient);
        } else {
            paint_ctx.fill(decrease_rect, &inactive_gradient);
        };

        // draw up and down triangles
        let mut increase_arrow = BezPath::new();
        increase_arrow.move_to(Point::new(4., height / 2. - 4.));
        increase_arrow.line_to(Point::new(width - 4., height / 2. - 4.));
        increase_arrow.line_to(Point::new(width / 2., 4.));
        increase_arrow.close_path();
        paint_ctx.fill(increase_arrow, &env.get(theme::LABEL_COLOR));

        let mut decrease_arrow = BezPath::new();
        decrease_arrow.move_to(Point::new(4., height / 2. + 4.));
        decrease_arrow.line_to(Point::new(width - 4., height / 2. + 4.));
        decrease_arrow.line_to(Point::new(width / 2., height - 4.));
        decrease_arrow.close_path();
        paint_ctx.fill(decrease_arrow, &env.get(theme::LABEL_COLOR));
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

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut f64, env: &Env) {
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

                let delay = Instant::now() + Duration::from_millis(500);
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
                let delay = Instant::now() + Duration::from_millis(200);
                self.timer_id = ctx.request_timer(delay);
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&f64>, _data: &f64, _env: &Env) {
        ctx.invalidate();
    }
}
