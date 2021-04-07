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

//! A slider widget.

use crate::kurbo::{Circle, Shape, Line};
use crate::widget::prelude::*;
use crate::{theme, LinearGradient, Point, Rect, UnitPoint, WidgetPod};
use druid_shell::piet::{Text, TextLayoutBuilder, TextLayout, FontFamily, PietTextLayout};

const TRACK_THICKNESS: f64 = 4.0;
const BORDER_WIDTH: f64 = 2.0;
const KNOB_STROKE_WIDTH: f64 = 2.0;

/// A slider, allowing interactive update of a numeric value.
///
/// This slider implements `Widget<f64>`, and works on values clamped
/// in the range `min..max`.
#[derive(Debug, Clone, Default)]
pub struct Slider {
    min: f64,
    max: f64,
    knob_pos: Point,
    knob_hovered: bool,
    x_offset: f64,
}

/// A range slider, allowing interactive update of two numeric values.
///
/// This slider implements `Widget<(f64, f64)>`, and works on values clamped
/// in the range `min..max`.
///
/// The first value of data is the lower bound, the second is the upper bound.
/// The first value is always lower than the second.
pub struct RangeSlider {
    min: f64,
    max: f64,
    min_range: f64,

    min_knob_pos: Point,
    min_knob_hovered: bool,
    min_knob_active: bool,

    max_knob_pos: Point,
    max_knob_hovered: bool,
    max_knob_active: bool,

    x_offset: f64,
}

/// A trait to access the range of a slider and do associated computations.
pub trait SliderBounds {
    /// The minimal possible value of a slider
    fn min(&self) -> f64;
    /// The maximal possible value of a slider
    fn max(&self) -> f64;

    ///
    fn normalize(&self, data: f64) -> f64 {
        (data.max(self.min()).min(self.max()) - self.min()) / (self.max() - self.min())
    }

    fn calculate_value(&self, mouse_x: f64, knob_width: f64, slider_width: f64) -> f64 {
        let scalar = ((mouse_x - knob_width / 2.) / (slider_width - knob_width))
            .max(0.0)
            .min(1.0);
        self.min() + scalar * (self.max() - self.min())
    }
}

impl SliderBounds for Slider {
    fn min(&self) -> f64 {
        self.min
    }

    fn max(&self) -> f64 {
        self.max
    }
}

impl SliderBounds for RangeSlider {
    fn min(&self) -> f64 {
        self.min
    }

    fn max(&self) -> f64 {
        self.max
    }
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

impl RangeSlider {
    /// Create a new `RangeSlider`.
    pub fn new() -> RangeSlider {
        RangeSlider {
            min: 0.,
            max: 1.,
            min_range: 0.,

            min_knob_pos: Default::default(),
            min_knob_active: Default::default(),
            min_knob_hovered: Default::default(),

            max_knob_pos: Default::default(),
            max_knob_hovered: false,
            max_knob_active: false,

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

    /// Builder-style method to set the minimal distance between the lower and upper bound.
    ///
    /// The value must be >= 0. The default value is `0.0`.
    pub fn with_min_distance(mut self, min_range: f64) -> Self {
        self.min_range = min_range.max(0.0);
        self
    }
}

impl Widget<f64> for Slider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let slider_width = ctx.size().width;

        match event {
            Event::MouseDown(mouse) => {
                ctx.set_active(true);
                if knob_hit_test(self.knob_pos, knob_size, mouse.pos) {
                    self.x_offset = self.knob_pos.x - mouse.pos.x
                } else {
                    self.x_offset = 0.;
                    *data = self.calculate_value(mouse.pos.x + self.x_offset, knob_size, slider_width);
                }
                ctx.request_paint();
            }
            Event::MouseUp(mouse) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = self.calculate_value(mouse.pos.x + self.x_offset, knob_size, slider_width);
                    ctx.request_paint();
                }
            }
            Event::MouseMove(mouse) => {
                if ctx.is_active() {
                    *data = self.calculate_value(mouse.pos.x + self.x_offset, knob_size, slider_width);
                    ctx.request_paint();
                }
                if ctx.is_hot() {
                    let knob_hover = knob_hit_test(self.knob_pos, knob_size, mouse.pos);
                    if knob_hover != self.knob_hovered {
                        self.knob_hovered = knob_hover;
                        ctx.request_paint();
                    }
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &f64, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &f64, _data: &f64, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &f64, env: &Env) -> Size {
        bc.debug_check("Slider");
        let height = env.get(theme::BASIC_WIDGET_HEIGHT);
        let width = env.get(theme::WIDE_WIDGET_WIDTH);
        let baseline_offset = (height / 2.0) - TRACK_THICKNESS;
        ctx.set_baseline_offset(baseline_offset);
        bc.constrain((width, height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, env: &Env) {
        draw_background(ctx, env);

        //Get ready to paint the knob
        let is_active = ctx.is_active();
        let is_hovered = self.knob_hovered;
        let clamped = self.normalize(*data);

        draw_knob(ctx, clamped, &mut self.knob_pos, is_hovered, is_active, env);
    }
}

impl Widget<(f64, f64)> for RangeSlider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut (f64, f64), env: &Env) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let slider_width = ctx.size().width;

        let mut mouse_position = None;

        match event {
            Event::MouseDown(me) => {
                ctx.set_active(true);
                // Check max first since it is painted above min
                if knob_hit_test(self.max_knob_pos, knob_size, me.pos) {
                    self.max_knob_active = true;
                    self.x_offset = self.max_knob_pos.x - me.pos.x;
                } else if knob_hit_test(self.min_knob_pos, knob_size, me.pos) {
                    self.min_knob_active = true;
                    self.x_offset = self.min_knob_pos.x - me.pos.x;
                } else {
                    let center = (self.min_knob_pos.x + self.max_knob_pos.x) / 2.0;
                    self.x_offset = 0.0;
                    mouse_position = Some(me.pos);
                    if me.pos.x <= center {
                        println!("select min: {} < {} ({}, {})", me.pos.x, center, self.min_knob_pos.x, self.max_knob_pos.x);
                        self.min_knob_active = true;
                    } else {
                        println!("select min: {} > {} ({}, {})", me.pos.x, center, self.min_knob_pos.x, self.max_knob_pos.x);
                        self.max_knob_active = true;
                    }
                }
            }
            Event::MouseMove(me) => {
                mouse_position = Some(me.pos);
                let new_min_hover = knob_hit_test(self.min_knob_pos, knob_size, me.pos);
                let new_max_hover = knob_hit_test(self.max_knob_pos, knob_size, me.pos);

                if new_min_hover != self.min_knob_hovered || new_max_hover != self.max_knob_hovered {
                    ctx.request_paint();
                }
                self.min_knob_hovered = new_min_hover;
                self.max_knob_hovered = new_max_hover;
            }
            Event::MouseUp(me) => {
                mouse_position = Some(me.pos);
            }
            _ => {}
        }

        if let Some(position) = mouse_position {
            if self.min_knob_active {
                data.0 = self.calculate_value(position.x + self.x_offset, knob_size, slider_width).min(data.1 - self.min_range);
            }
            if self.max_knob_active {
                data.1 = self.calculate_value(position.x + self.x_offset, knob_size, slider_width).max(data.0 + self.min_range);
            }
        }

        if let Event::MouseUp(_) = event {
            if self.min_knob_active || self.max_knob_active {
                ctx.request_paint();
            }

            self.min_knob_active = false;
            self.max_knob_active = false;
            ctx.set_active(false);
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &(f64, f64), _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &(f64, f64), _data: &(f64, f64), _env: &Env) {
        ctx.request_paint();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &(f64, f64), env: &Env) -> Size {
        bc.debug_check("Slider");
        let height = env.get(theme::BASIC_WIDGET_HEIGHT);
        let width = env.get(theme::WIDE_WIDGET_WIDTH);
        let baseline_offset = (height / 2.0) - TRACK_THICKNESS;
        ctx.set_baseline_offset(baseline_offset);
        bc.constrain((width, height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &(f64, f64), env: &Env) {
        draw_background(ctx, env);

        //Get ready to paint the left knob
        let is_active = self.min_knob_active;
        let is_hovered = self.min_knob_hovered;
        let clamped = self.normalize(data.0);

        draw_knob(ctx, clamped, &mut self.min_knob_pos, is_hovered, is_active, env);

        //Get ready to paint the right knob
        let is_active = self.max_knob_active;
        let is_hovered = self.max_knob_hovered;
        let clamped = self.normalize(data.1);

        draw_knob(ctx, clamped, &mut self.max_knob_pos, is_hovered, is_active, env);
    }
}

fn draw_knob(ctx: &mut PaintCtx, clamped: f64, pos: &mut Point, is_hovered: bool, is_active: bool, env: &Env) {
    let rect = ctx.size().to_rect();
    let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
    let knob_position = (rect.width() - knob_size) * clamped + knob_size / 2.;
    *pos = Point::new(knob_position, knob_size / 2.);
    let knob_circle = Circle::new(*pos, (knob_size - KNOB_STROKE_WIDTH) / 2.);

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

    ctx.stroke(knob_circle, &border_color, KNOB_STROKE_WIDTH);

    //Actually paint the knob
    ctx.fill(knob_circle, &knob_gradient);
}

fn draw_background(ctx: &mut PaintCtx, env: &Env) {
    let rect = ctx.size().to_rect();
    let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
    let background_width = rect.width() - knob_size;
    let background_origin = Point::new(knob_size / 2., (knob_size - TRACK_THICKNESS) / 2.);
    let background_size = Size::new(background_width, TRACK_THICKNESS);
    let background_rect = Rect::from_origin_size(background_origin, background_size)
        .inset(-BORDER_WIDTH / 2.)
        .to_rounded_rect(2.);

    let background_gradient = LinearGradient::new(
        UnitPoint::TOP,
        UnitPoint::BOTTOM,
        (
            env.get(theme::BACKGROUND_LIGHT),
            env.get(theme::BACKGROUND_DARK),
        ),
    );

    ctx.stroke(background_rect, &env.get(theme::BORDER_DARK), BORDER_WIDTH);

    ctx.fill(background_rect, &background_gradient);
}

fn knob_hit_test(knob_pos: Point, knob_width: f64, mouse_pos: Point) -> bool {
    let knob_circle = Circle::new(knob_pos, knob_width / 2.);
    knob_circle.winding(mouse_pos) > 0
}

/// A widget to annotate the values of the inner slider.
pub struct SliderAnnotation<T, W> {
    slider: WidgetPod<T, W>,
    step_size: f64,
    sub_steps: u64,

    //Calculated
    rebuild: bool,
    annotations: Vec<PietTextLayout>,
    first: f64,
}

impl<T: Data, W: Widget<T> + SliderBounds> SliderAnnotation<T, W> {
    /// Creates a new annotated slider.
    /// The widget will annotate at each multiple of `step_size` in the range of min and max.
    /// sub_steps is the number of unannotated lines between each annotation.
    pub fn new(widget: W, step_size: f64, sub_steps: u64) -> Self {
        SliderAnnotation {
            slider: WidgetPod::new(widget),
            step_size,
            sub_steps,
            rebuild: true,
            annotations: vec![],
            first: 0.0,
        }
    }
    fn build_annotation(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let inner = self.slider.widget();
        let low = (inner.min() / self.step_size).ceil() * self.step_size;
        let high = (inner.max() / self.step_size).floor() * self.step_size;

        self.annotations.clear();

        let mut current = low;
        while current <= high {
            let text = ctx.text().new_text_layout(format!("{}", current))
                .font(FontFamily::SANS_SERIF, 15.0)
                .text_color(env.get(theme::LABEL_COLOR))
                .build()
                .unwrap();

            self.annotations.push(text);

            current += self.step_size;
        }

        self.rebuild = false;
    }

    /// Returns the inner slider.
    pub fn slider(&self) -> &W {
        self.slider.widget()
    }

    /// Returns a mutable reference to the inner slider.
    /// The annotations are marked as invalidated and will be rebuild before the next paint call.
    pub fn slider_mut(&mut self) -> &mut W {
        self.rebuild = true;
        self.slider.widget_mut()
    }
}

impl<T: Data, W: Widget<T> + SliderBounds> Widget<T> for SliderAnnotation<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.slider.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.slider.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        self.slider.update(ctx, data, env);
        if ctx.env_changed() {
            self.rebuild = true;
            ctx.request_paint();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let slider_size = self.slider.layout(ctx, &bc.shrink((0.0, 30.0)), data, env);
        self.slider.set_origin(ctx, data, env, Point::ZERO);
        ctx.set_baseline_offset(self.slider.baseline_offset() + 30.0);
        slider_size + Size::new(0.0, 30.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let rect = ctx.size().to_rect();
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);

        if self.rebuild {
            self.build_annotation(ctx, env);
        }

        let slider = self.slider.widget();
        let small_step = self.step_size / (self.sub_steps + 1) as f64;
        let top = self.slider.layout_rect().y1;
        let small_bottom = top + 5.0;
        let large_bottom = top + 10.0;
        let color = env.get(theme::LABEL_COLOR);

        let mut current = self.first - small_step;

        let min = slider.min();
        let max = slider.max();
        while current >= min {
            let knob_position = (rect.width() - knob_size) * slider.normalize(current) + knob_size / 2.;
            ctx.stroke(Line::new((knob_position, top), (knob_position, small_bottom)), &color, 1.0);
            current -= small_step;
        }

        current = self.first;

        for layout in &self.annotations {
            let knob_position = (rect.width() - knob_size) * slider.normalize(current) + knob_size / 2.;
            ctx.stroke(Line::new((knob_position, top), (knob_position, large_bottom)), &color, 1.0);
            let text_pos = Point::new(knob_position - layout.size().width / 2.0, large_bottom);
            ctx.draw_text(layout, text_pos);

            let mut sub = current;

            for _ in 0..self.sub_steps {
                sub += small_step;
                if sub > max {
                    break;
                }
                let knob_position = (rect.width() - knob_size) * slider.normalize(sub) + knob_size / 2.;
                ctx.stroke(Line::new((knob_position, top), (knob_position, small_bottom)), &color, 1.0);
            }

            current += self.step_size;
        }

        self.slider.paint(ctx, data, env);
    }
}