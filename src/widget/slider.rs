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

//! A slider widget.

use crate::kurbo::{Circle, Line, Point, Rect, Size, Vec2};
use crate::piet::{Color, FillRule, LineCap, RenderContext, StrokeStyle};
use crate::{
    Action, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

const KNOB_WIDTH: f64 = 24.;
const BACKGROUND_THICKNESS: f64 = 4.;
const BACKGROUND_COLOR: Color = Color::rgb24(0x55_55_55);
const KNOB_COLOR: Color = Color::rgb24(0xf0_f0_e5);
const KNOB_HOVER_COLOR: Color = Color::rgb24(0xa0_a0_a5);
const KNOB_PRESSED_COLOR: Color = Color::rgb24(0x75_75_75);

/// A slider, allowing interactive update of a numeric value.
#[derive(Debug, Clone, Default)]
pub struct Slider {
    width: f64,
    knob_pos: Point,
    knob_hovered: bool,
    x_offset: f64,
}

impl Slider {
    fn knob_hit_test(&self, knob_width: f64, mouse_pos: Point) -> bool {
        let knob_circle = Circle::new(self.knob_pos, knob_width / 2.);
        if mouse_pos.distance(knob_circle.center) < knob_circle.radius {
            return true;
        }
        false
    }

    fn calculate_value(&self, mouse_x: f64, knob_width: f64) -> f64 {
        ((mouse_x + self.x_offset - KNOB_WIDTH / 2.) / (self.width - knob_width))
            .max(0.0)
            .min(1.0)
    }
}

impl Widget<f64> for Slider {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &f64, _env: &Env) {
        let clamped = data.max(0.0).min(1.0);
        let rect = Rect::from_origin_size(Point::ORIGIN, base_state.size());

        //Store the width so we can calulate slider position from mouse events
        self.width = rect.width();

        //Paint the background
        let background_width = rect.width() - KNOB_WIDTH;
        let background_origin = Point::new(KNOB_WIDTH / 2., rect.height() / 2.);
        let background_line = Line::new(
            background_origin,
            background_origin + Vec2::new(background_width, 0.),
        );

        let brush = paint_ctx.solid_brush(BACKGROUND_COLOR);
        let mut stroke = StrokeStyle::new();
        stroke.set_line_cap(LineCap::Round);
        paint_ctx.stroke(background_line, &brush, BACKGROUND_THICKNESS, Some(&stroke));

        //Paint the slider
        let is_active = base_state.is_active();

        let knob_color = match (is_active, self.knob_hovered) {
            (true, _) => KNOB_PRESSED_COLOR,
            (false, true) => KNOB_HOVER_COLOR,
            _ => KNOB_COLOR,
        };

        let knob_position = (self.width - KNOB_WIDTH) * clamped + KNOB_WIDTH / 2.;
        self.knob_pos = Point::new(knob_position, rect.height() / 2.);
        let knob_circle = Circle::new(self.knob_pos, KNOB_WIDTH / 2.);
        let brush = paint_ctx.solid_brush(knob_color);
        paint_ctx.fill(knob_circle, &brush, FillRule::NonZero);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        _env: &Env,
    ) -> Size {
        bc.constrain(bc.max())
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut f64,
        _env: &Env,
    ) -> Option<Action> {
        match event {
            Event::MouseDown(mouse) => {
                ctx.set_active(true);
                if self.knob_hit_test(KNOB_WIDTH, mouse.pos) {
                    self.x_offset = self.knob_pos.x - mouse.pos.x
                } else {
                    self.x_offset = 0.;
                    *data = self.calculate_value(mouse.pos.x, KNOB_WIDTH);
                }
                ctx.invalidate();
            }
            Event::MouseUp(mouse) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = self.calculate_value(mouse.pos.x, KNOB_WIDTH);
                    ctx.invalidate();
                }
            }
            Event::MouseMoved(mouse) => {
                if ctx.is_active() {
                    *data = self.calculate_value(mouse.pos.x, KNOB_WIDTH);
                }
                if ctx.is_hot() {
                    if self.knob_hit_test(KNOB_WIDTH, mouse.pos) {
                        self.knob_hovered = true
                    } else {
                        self.knob_hovered = false
                    }
                }
                ctx.invalidate();
            }
            _ => (),
        }
        None
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&f64>, _data: &f64, _env: &Env) {
        ctx.invalidate();
    }
}
