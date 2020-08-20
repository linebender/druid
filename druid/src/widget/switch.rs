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

//! A toggle switch widget.

use std::time::Duration;

use crate::kurbo::{Circle, Point, Rect, Shape, Size};
use crate::piet::{
    FontBuilder, LinearGradient, RenderContext, Text, TextLayout, TextLayoutBuilder, UnitPoint,
};
use crate::theme;
use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, UpdateCtx,
    Widget,
};

const SWITCH_CHANGE_TIME: f64 = 0.2;
const SWITCH_PADDING: f64 = 3.;
const SWITCH_WIDTH_RATIO: f64 = 2.75;

/// A switch that toggles a `bool`.
#[derive(Debug, Clone, Default)]
pub struct Switch {
    knob_pos: Point,
    knob_hovered: bool,
    knob_dragged: bool,
    animation_in_progress: bool,
}

impl Switch {
    /// Create a new `Switch`.
    pub fn new() -> Switch {
        Self::default()
    }

    fn knob_hit_test(&self, knob_width: f64, mouse_pos: Point) -> bool {
        let knob_circle = Circle::new(self.knob_pos, knob_width / 2.);
        knob_circle.winding(mouse_pos) > 0
    }

    fn paint_labels(&mut self, ctx: &mut PaintCtx, env: &Env, switch_width: f64) {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let knob_size = switch_height - 2. * SWITCH_PADDING;

        let font = ctx
            .text()
            .new_font_by_name(font_name, font_size)
            .build()
            .unwrap();

        // off/on labels
        // TODO: use LocalizedString
        let on_label_layout = ctx
            .text()
            .new_text_layout(&font, "ON", std::f64::INFINITY)
            .build()
            .unwrap();

        let off_label_layout = ctx
            .text()
            .new_text_layout(&font, "OFF", std::f64::INFINITY)
            .build()
            .unwrap();

        // position off/on labels
        let mut on_label_origin = UnitPoint::LEFT.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                (ctx.size().width - on_label_layout.width()).max(0.0),
                switch_height + (font_size * 1.2) / 2.,
            ),
        ));

        let mut off_label_origin = UnitPoint::LEFT.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                (ctx.size().width - off_label_layout.width()).max(0.0),
                switch_height + (font_size * 1.2) / 2.,
            ),
        ));

        // adjust label position
        on_label_origin.y = on_label_origin.y.min(switch_height);
        off_label_origin.y = off_label_origin.y.min(switch_height);

        on_label_origin.x = self.knob_pos.x - switch_width + knob_size;
        off_label_origin.x = switch_width - off_label_layout.width() - SWITCH_PADDING * 2.
            + self.knob_pos.x
            - knob_size / 2.
            - SWITCH_PADDING;

        ctx.draw_text(
            &on_label_layout,
            on_label_origin,
            &env.get(theme::LABEL_COLOR),
        );
        ctx.draw_text(
            &off_label_layout,
            off_label_origin,
            &env.get(theme::LABEL_COLOR),
        );
    }
}

impl Widget<bool> for Switch {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, env: &Env) {
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let switch_width = switch_height * SWITCH_WIDTH_RATIO;
        let knob_size = switch_height - 2. * SWITCH_PADDING;
        let on_pos = switch_width - knob_size / 2. - SWITCH_PADDING;
        let off_pos = knob_size / 2. + SWITCH_PADDING;

        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if self.knob_dragged {
                    // toggle value when dragging if knob has been moved far enough
                    *data = self.knob_pos.x > switch_width / 2.;
                } else if ctx.is_active() {
                    // toggle value on click
                    *data = !*data;
                }

                ctx.set_active(false);

                self.knob_dragged = false;
                self.animation_in_progress = true;
                ctx.request_anim_frame();
            }
            Event::MouseMove(mouse) => {
                if ctx.is_active() {
                    self.knob_pos.x = mouse.pos.x.min(on_pos).max(off_pos);
                    self.knob_dragged = true;
                }
                if ctx.is_hot() {
                    self.knob_hovered = self.knob_hit_test(knob_size, mouse.pos)
                }
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &bool, env: &Env) {
        if let LifeCycle::AnimFrame(interval) = event {
            let delta = Duration::from_nanos(*interval).as_secs_f64();
            let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
            let switch_width = switch_height * SWITCH_WIDTH_RATIO;
            let knob_size = switch_height - 2. * SWITCH_PADDING;
            let on_pos = switch_width - knob_size / 2. - SWITCH_PADDING;
            let off_pos = knob_size / 2. + SWITCH_PADDING;

            // move knob to right position depending on the value
            if self.animation_in_progress {
                let change_time = if *data {
                    SWITCH_CHANGE_TIME
                } else {
                    -SWITCH_CHANGE_TIME
                };
                let change = (switch_width / change_time) * delta;
                self.knob_pos.x = (self.knob_pos.x + change).min(on_pos).max(off_pos);

                if self.knob_pos.x > off_pos && self.knob_pos.x < on_pos {
                    ctx.request_anim_frame();
                } else {
                    self.animation_in_progress = false;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, _env: &Env) {
        if old_data != data {
            self.animation_in_progress = true;
            ctx.request_anim_frame();
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &bool,
        env: &Env,
    ) -> Size {
        let width = env.get(theme::BORDERED_WIDGET_HEIGHT) * SWITCH_WIDTH_RATIO;
        bc.constrain(Size::new(width, env.get(theme::BORDERED_WIDGET_HEIGHT)))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, env: &Env) {
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let switch_width = switch_height * SWITCH_WIDTH_RATIO;
        let knob_size = switch_height - 2. * SWITCH_PADDING;
        let on_pos = switch_width - knob_size / 2. - SWITCH_PADDING;
        let off_pos = knob_size / 2. + SWITCH_PADDING;
        let stroke_width = 2.0;

        let background_rect = Size::new(switch_width, switch_height)
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(switch_height / 2.);

        // position knob
        if !self.animation_in_progress && !self.knob_dragged {
            if *data {
                self.knob_pos.x = on_pos;
            } else {
                self.knob_pos.x = off_pos;
            }
        };

        self.knob_pos = Point::new(self.knob_pos.x, knob_size / 2. + SWITCH_PADDING);
        let knob_circle = Circle::new(self.knob_pos, knob_size / 2.);

        // paint different background for on and off state
        // opacity of background color depends on knob position
        // todo: make color configurable
        let opacity = (self.knob_pos.x - off_pos) / (on_pos - off_pos);

        let background_gradient_on_state = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::PRIMARY_LIGHT).with_alpha(opacity),
                env.get(theme::PRIMARY_DARK).with_alpha(opacity),
            ),
        );
        let background_gradient_off_state = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT).with_alpha(1. - opacity),
                env.get(theme::BACKGROUND_DARK).with_alpha(1. - opacity),
            ),
        );

        ctx.stroke(background_rect, &env.get(theme::BORDER_DARK), stroke_width);
        ctx.fill(background_rect, &background_gradient_on_state);
        ctx.fill(background_rect, &background_gradient_off_state);
        ctx.clip(background_rect);

        // paint the knob
        let is_active = ctx.is_active();
        let is_hovered = self.knob_hovered;

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

        ctx.stroke(knob_circle, &border_color, 2.);
        ctx.fill(knob_circle, &knob_gradient);

        // paint on/off label
        self.paint_labels(ctx, env, switch_width);
    }
}
