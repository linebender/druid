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

use crate::kurbo::{Circle, Shape};
use crate::piet::{LinearGradient, RenderContext, UnitPoint};
use crate::widget::prelude::*;
use crate::{theme, ArcStr, Point, TextLayout};

const SWITCH_CHANGE_TIME: f64 = 0.2;
const SWITCH_PADDING: f64 = 3.;
const SWITCH_WIDTH_RATIO: f64 = 2.75;

/// A switch that toggles a `bool`.
#[derive(Debug, Clone)]
pub struct Switch {
    knob_pos: Point,
    knob_hovered: bool,
    knob_dragged: bool,
    animation_in_progress: bool,
    on_text: TextLayout<ArcStr>,
    off_text: TextLayout<ArcStr>,
}

impl Default for Switch {
    fn default() -> Self {
        Switch {
            knob_pos: Point::ZERO,
            knob_hovered: false,
            knob_dragged: false,
            animation_in_progress: false,
            //TODO: use localized strings, also probably make these configurable?
            on_text: TextLayout::from_text("ON"),
            off_text: TextLayout::from_text("OFF"),
        }
    }
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
        let switch_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let knob_size = switch_height - 2. * SWITCH_PADDING;

        let on_size = self.on_text.size();
        let off_size = self.off_text.size();

        let label_y = (switch_height - on_size.height).max(0.0) / 2.0;
        let label_x_space = switch_width - knob_size - SWITCH_PADDING * 2.0;
        let off_pos = knob_size / 2. + SWITCH_PADDING;
        let knob_delta = self.knob_pos.x - off_pos;

        let on_label_width = on_size.width;
        let on_base_x_pos =
            -on_label_width - (label_x_space - on_label_width) / 2.0 + SWITCH_PADDING;
        let on_label_origin = Point::new(on_base_x_pos + knob_delta, label_y);

        let off_base_x_pos = knob_size + (label_x_space - off_size.width) / 2.0 + SWITCH_PADDING;
        let off_label_origin = Point::new(off_base_x_pos + knob_delta, label_y);

        self.on_text.draw(ctx, on_label_origin);
        self.off_text.draw(ctx, off_label_origin);
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
            Event::AnimFrame(interval) => {
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

                    if (self.knob_pos.x > off_pos && !*data) || (self.knob_pos.x < on_pos && *data)
                    {
                        ctx.request_anim_frame();
                    } else {
                        self.animation_in_progress = false;
                    }
                    ctx.request_paint();
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &bool, env: &Env) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.on_text.rebuild_if_needed(ctx.text(), env);
            self.off_text.rebuild_if_needed(ctx.text(), env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, _env: &Env) {
        if old_data != data {
            self.animation_in_progress = true;
            ctx.request_anim_frame();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _: &bool, env: &Env) -> Size {
        let text_metrics = self.on_text.layout_metrics();
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let width = height * SWITCH_WIDTH_RATIO;

        let label_y = (height - text_metrics.size.height).max(0.0) / 2.0;
        let text_bottom_padding = height - (text_metrics.size.height + label_y);
        let text_baseline_offset = text_metrics.size.height - text_metrics.first_baseline;
        ctx.set_baseline_offset(text_bottom_padding + text_baseline_offset);
        bc.constrain(Size::new(width, height))
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
