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

//! A toggle switch widget.

use crate::kurbo::{Circle, Point, Rect, RoundedRect, Size};
use crate::piet::{LinearGradient, RenderContext, UnitPoint};
use crate::theme;
use crate::widget::Align;
use crate::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

#[derive(Debug, Clone)]
pub struct Switch;

impl Switch {
    pub fn new() -> impl Widget<bool> { Align::vertical(UnitPoint::CENTER, SwitchRaw::default()) }
}

#[derive(Debug, Clone, Default)]
pub struct SwitchRaw {
    knob_pos: Point,
    knob_hovered: bool,
}

impl SwitchRaw {
    fn knob_hit_test(&self, knob_width: f64, mouse_pos: Point) -> bool {
        let knob_circle = Circle::new(self.knob_pos, knob_width / 2.);
        if mouse_pos.distance(knob_circle.center) < knob_circle.radius {
            return true;
        }
        false
    }
}

impl Widget<bool> for SwitchRaw {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &bool, env: &Env) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let switch_thickness = 8. + knob_size;

        let background_rect =
            RoundedRect::from_origin_size(Point::ORIGIN, Size::new(switch_thickness * 2., switch_thickness).to_vec2(), switch_thickness / 2.);

        let background_gradient = if *data {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::PRIMARY_LIGHT),
                    env.get(theme::PRIMARY_DARK),
                ),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::BACKGROUND_LIGHT),
                    env.get(theme::BACKGROUND_DARK),
                ),
            )
        };



        paint_ctx.stroke(background_rect, &env.get(theme::BORDER), 2.0);

        paint_ctx.fill(background_rect, &background_gradient);

        let is_active = base_state.is_active();
        let is_hovered = self.knob_hovered;

        let knob_position = if *data {
            switch_thickness * 2. - knob_size / 2. - 4.
        } else {
            knob_size / 2. + 4.
        };

        self.knob_pos = Point::new(knob_position, knob_size / 2. + 4.);
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

        paint_ctx.fill(knob_circle, &knob_gradient);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &bool,
        env: &Env,
    ) -> Size {
        let width = (8. + env.get(theme::BASIC_WIDGET_HEIGHT)) * 2.;
        bc.constrain(Size::new(
            width,
            env.get(theme::BASIC_WIDGET_HEIGHT),
        ))
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut bool, env: &Env) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);

        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        if *data {
                            *data = false;
                        } else {
                            *data = true;
                        }
                    }
                    ctx.invalidate();
                }
            }
            Event::MouseMoved(mouse) => {
                if ctx.is_active() {
                    // todo
                }
                if ctx.is_hot() {
                    if self.knob_hit_test(knob_size, mouse.pos) {
                        self.knob_hovered = true
                    } else {
                        self.knob_hovered = false
                    }
                }
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&bool>, _data: &bool, _env: &Env) {
        ctx.invalidate();
    }
}