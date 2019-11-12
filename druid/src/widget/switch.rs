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

use crate::kurbo::{Circle, Point, Rect, RoundedRect, Shape, Size};
use crate::piet::{
    FontBuilder, LinearGradient, RenderContext, Text, TextLayout, TextLayoutBuilder, UnitPoint,
};
use crate::theme;
use crate::widget::Align;
use crate::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

const SWITCH_PADDING: f64 = 3.;
const SWITCH_WIDTH_RATIO: f64 = 2.75;

/// A switch that toggles a boolean.
#[derive(Debug, Clone, Default)]
pub struct Switch {
    knob_pos: Point,
    knob_hovered: bool,
    knob_x_offset: f64,
    is_dragging: bool,
    is_animated: bool,
}

impl Switch {
    pub fn new() -> impl Widget<bool> {
        Align::vertical(UnitPoint::CENTER, Self::default())
    }

    fn knob_hit_test(&self, knob_width: f64, mouse_pos: Point) -> bool {
        let knob_circle = Circle::new(self.knob_pos, knob_width / 2.);
        knob_circle.winding(mouse_pos) > 0
    }

    fn paint_label(
        &mut self,
        paint_ctx: &mut PaintCtx,
        base_state: &BaseState,
        data: bool,
        env: &Env,
        switch_width: f64,
    ) {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let switch_width = switch_height * SWITCH_WIDTH_RATIO;
        let knob_size = switch_height - 2. * SWITCH_PADDING;
        let on_pos = switch_width - knob_size / 2. - SWITCH_PADDING;
        let off_pos = knob_size / 2. + SWITCH_PADDING;

        // TODO: use LocalizedString
        let on_label = "ON";
        let off_label = "OFF";

        let font = paint_ctx
            .text()
            .new_font_by_name(font_name, font_size)
            .build()
            .unwrap();

        let on_text_layout = paint_ctx
            .text()
            .new_text_layout(&font, on_label)
            .build()
            .unwrap();

        let off_text_layout = paint_ctx
            .text()
            .new_text_layout(&font, off_label)
            .build()
            .unwrap();

        let mut on_origin = UnitPoint::LEFT.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                (base_state.size().width - on_text_layout.width()).max(0.0),
                switch_height + (font_size * 1.2) / 2.,
            ),
        ));

        let mut off_origin = UnitPoint::LEFT.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                (base_state.size().width - off_text_layout.width()).max(0.0),
                switch_height + (font_size * 1.2) / 2.,
            ),
        ));

        // adjust label position
        on_origin.y = on_origin.y.min(switch_height);
        off_origin.y = off_origin.y.min(switch_height);

        on_origin.x = self.knob_x_offset - switch_width + knob_size;
        off_origin.x = switch_width - off_text_layout.width() - SWITCH_PADDING * 2.
            + self.knob_x_offset
            - knob_size / 2.
            - SWITCH_PADDING;

        paint_ctx.draw_text(&on_text_layout, on_origin, &env.get(theme::LABEL_COLOR));
        paint_ctx.draw_text(&off_text_layout, off_origin, &env.get(theme::LABEL_COLOR));
    }
}

impl Widget<bool> for Switch {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &bool, env: &Env) {
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let switch_width = switch_height * SWITCH_WIDTH_RATIO;
        let knob_size = switch_height - 2. * SWITCH_PADDING;

        let background_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            Size::new(switch_width, switch_height).to_vec2(),
            switch_height / 2.,
        );

        // paint different background for on and off state
        // todo: make color configurable
        let background_gradient = if *data {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::PRIMARY_LIGHT), env.get(theme::PRIMARY_DARK)),
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
        paint_ctx.clip(background_rect);

        // paint the knob
        let is_active = base_state.is_active();
        let is_hovered = self.knob_hovered;
        let on_pos = switch_width - knob_size / 2. - SWITCH_PADDING;
        let off_pos = knob_size / 2. + SWITCH_PADDING;

        let knob_position = if self.is_animated || self.is_dragging {
            self.knob_x_offset
        } else {
            if *data {
                self.knob_x_offset = on_pos;
                on_pos
            } else {
                self.knob_x_offset = off_pos;
                off_pos
            }
        };

        self.knob_pos = Point::new(knob_position, knob_size / 2. + SWITCH_PADDING);
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

        // paint the border
        let border_color = if is_hovered || is_active {
            env.get(theme::FOREGROUND_LIGHT)
        } else {
            env.get(theme::FOREGROUND_DARK)
        };

        paint_ctx.stroke(knob_circle, &border_color, 2.);
        paint_ctx.fill(knob_circle, &knob_gradient);

        // paint on/off label
        self.paint_label(paint_ctx, base_state, *data, env, switch_width);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &bool,
        env: &Env,
    ) -> Size {
        let width =
            (2. * SWITCH_PADDING + env.get(theme::BORDERED_WIDGET_HEIGHT)) * SWITCH_WIDTH_RATIO;
        bc.constrain(Size::new(width, env.get(theme::BORDERED_WIDGET_HEIGHT)))
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut bool, env: &Env) {
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let switch_width = switch_height * SWITCH_WIDTH_RATIO;
        let knob_size = switch_height - 2. * SWITCH_PADDING;
        let on_pos = switch_width - knob_size / 2. - SWITCH_PADDING;
        let off_pos = knob_size / 2. + SWITCH_PADDING;

        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                ctx.set_active(false);

                if self.is_dragging {
                    *data = self.knob_x_offset > switch_width / 2.;
                } else {
                    *data = !*data;
                }

                ctx.invalidate();
                self.is_dragging = false;
                self.is_animated = true;
                ctx.request_anim_frame();
            }
            Event::MouseMoved(mouse) => {
                if ctx.is_active() {
                    self.knob_x_offset = mouse.pos.x.min(on_pos).max(off_pos);
                    self.is_dragging = true;
                }
                if ctx.is_hot() {
                    self.knob_hovered = self.knob_hit_test(knob_size, mouse.pos)
                }
                ctx.invalidate();
            }
            Event::AnimFrame(_) => {
                if self.is_animated {
                    let delta = if *data { 2. } else { -2. };
                    self.knob_x_offset = self.knob_x_offset + delta;

                    if self.knob_x_offset > off_pos && self.knob_x_offset < on_pos {
                        ctx.request_anim_frame();
                    } else {
                        self.is_animated = false;
                    }
                }
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&bool>, _data: &bool, _env: &Env) {
        ctx.invalidate();
    }
}
