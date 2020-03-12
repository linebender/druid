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

use crate::kurbo::{Circle, Point, Rect, RoundedRect, Shape, Size};
use crate::theme;
use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, LinearGradient,
    PaintCtx, RenderContext, UnitPoint, UpdateCtx, Widget,
};

/// A slider, allowing interactive update of a numeric value.
///
/// This slider implements `Widget<f64>`, and works on values clamped
/// in the range 0..1.0.
#[derive(Debug, Clone, Default)]
pub struct Slider {
    min: f64,
    max: f64,
    knob_pos: Point,
    knob_hovered: bool,
    x_offset: f64,
}

impl Slider {
    /// Create a new `Slider`.
    pub fn new() -> Slider {
        Slider {
            min: 0.,
            max: 1.,
            knob_pos: Default::default(),
            knob_hovered: Default::default(),
            x_offset: Default::default(),
        }
    }

/// Builder-style method to set the range covered by this slider.
///
/// The default range is `0.0..1.0`. 
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }
}

impl Slider {
    fn knob_hit_test(&self, knob_width: f64, mouse_pos: Point) -> bool {
        let knob_circle = Circle::new(self.knob_pos, knob_width / 2.);
        knob_circle.winding(mouse_pos) > 0
    }

    fn calculate_value(&self, mouse_x: f64, knob_width: f64, slider_width: f64) -> f64 {
        let scalar = ((mouse_x + self.x_offset - knob_width / 2.) / (slider_width - knob_width))
            .max(0.0)
            .min(1.0);
        self.min + scalar * (self.max - self.min)
    }
}

impl Widget<f64> for Slider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let slider_width = ctx.size().width;

        match event {
            Event::MouseDown(mouse) => {
                ctx.set_active(true);
                if self.knob_hit_test(knob_size, mouse.pos) {
                    self.x_offset = self.knob_pos.x - mouse.pos.x
                } else {
                    self.x_offset = 0.;
                    *data = self.calculate_value(mouse.pos.x, knob_size, slider_width);
                }
                ctx.request_paint();
            }
            Event::MouseUp(mouse) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = self.calculate_value(mouse.pos.x, knob_size, slider_width);
                    ctx.request_paint();
                }
            }
            Event::MouseMoved(mouse) => {
                if ctx.is_active() {
                    *data = self.calculate_value(mouse.pos.x, knob_size, slider_width);
                }
                if ctx.is_hot() {
                    if self.knob_hit_test(knob_size, mouse.pos) {
                        self.knob_hovered = true
                    } else {
                        self.knob_hovered = false
                    }
                }
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &f64, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &f64, _data: &f64, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        env: &Env,
    ) -> Size {
        bc.debug_check("Slider");
        let height = env.get(theme::BASIC_WIDGET_HEIGHT);
        let width = env.get(theme::WIDE_WIDGET_WIDTH);
        bc.constrain((width, height))
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &f64, env: &Env) {
        let clamped = data.max(0.0).min(1.0);
        let rect = Rect::from_origin_size(Point::ORIGIN, paint_ctx.size());
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let track_thickness = 4.;

        //Paint the background
        let background_width = rect.width() - knob_size;
        let background_origin = Point::new(knob_size / 2., (knob_size - track_thickness) / 2.);
        let background_size = Size::new(background_width, track_thickness);
        let background_rect =
            RoundedRect::from_origin_size(background_origin, background_size.to_vec2(), 2.);

        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        paint_ctx.stroke(background_rect, &env.get(theme::BORDER_DARK), 2.0);

        paint_ctx.fill(background_rect, &background_gradient);

        //Get ready to paint the knob
        let is_active = paint_ctx.is_active();
        let is_hovered = self.knob_hovered;

        let knob_position = (rect.width() - knob_size) * clamped + knob_size / 2.;
        self.knob_pos = Point::new(knob_position, knob_size / 2.);
        let knob_circle = Circle::new(self.knob_pos, knob_size / 2.);

        let normal_knob_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::FOREGROUND_LIGHT),
                env.get(theme::FOREGROUND_DARK),
            ),
        );
        let flipped_knob_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::FOREGROUND_DARK),
                env.get(theme::FOREGROUND_LIGHT),
            ),
        );

        let knob_gradient = if is_active {
            flipped_knob_gradient
        } else {
            normal_knob_gradient
        };

        //Paint the border
        let border_color = if is_hovered || is_active {
            env.get(theme::FOREGROUND_LIGHT)
        } else {
            env.get(theme::FOREGROUND_DARK)
        };

        paint_ctx.stroke(knob_circle, &border_color, 2.);

        //Actually paint the knob
        paint_ctx.fill(knob_circle, &knob_gradient);
    }
}
