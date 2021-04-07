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
use druid_shell::piet::{Text, TextLayoutBuilder, TextLayout, FontFamily, PietTextLayout, PietText};

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
    fill_track: bool,
    snap: Option<f64>,

    head: SliderHead,
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
    fill_track: bool,
    snap: Option<f64>,

    min_knob: SliderHead,
    max_knob: SliderHead,
}

/// A trait to access the range of a slider and do associated computations.
pub trait SliderBounds {
    /// The minimal possible value of a slider
    fn min(&self) -> f64;
    /// The maximal possible value of a slider
    fn max(&self) -> f64;

    /// converts a value to proportion of the range `min()..max()`.
    fn normalize(&self, data: f64) -> f64 {
        (data.max(self.min()).min(self.max()) - self.min()) / (self.max() - self.min())
    }

    /// converts a x-coordinate on the slider into the corresponding value.
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

impl SliderBounds for (f64, f64) {
    fn min(&self) -> f64 {self.0}

    fn max(&self) -> f64 {self.1}
}

impl Slider {
    /// Create a new `Slider`.
    pub fn new() -> Slider {
        Slider {
            min: 0.,
            max: 1.,
            fill_track: false,
            snap: None,
            head: SliderHead::new(),
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

    /// Fill the active area with the foreground color.
    pub fn view_track(mut self) -> Self {
        self.fill_track = true;
        self
    }

    /// snaps the slider to values at the given precision
    pub fn snap(mut self, snap: f64) -> Self {
        self.snap = Some(snap.abs());
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
            fill_track: false,

            snap: None,
            min_knob: SliderHead::new(),
            max_knob: SliderHead::new(),
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

    /// Fill the active area with the foreground color.
    pub fn view_track(mut self) -> Self {
        self.fill_track = true;
        self
    }

    /// snaps the slider to values at the given precision
    pub fn snap(mut self, snap: f64) -> Self {
        self.snap = Some(snap.abs());
        self
    }
}

impl Widget<f64> for Slider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        self.head.handle_event(ctx, event, data, env, (self.min, self.max));

        if let Some(snap) = self.snap {
            *data = (*data / snap).round() * snap;
        }

        if !self.head.is_active() {
            if let Event::MouseDown(me) = event {
                let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
                let slider_width = ctx.size().width;

                ctx.set_active(true);
                self.head.x_offset = 0.0;
                self.head.is_active = true;
                self.head.pos = me.pos;
                *data = self.calculate_value(me.pos.x, knob_size, slider_width);
                if let Some(snap) = self.snap {
                    *data = (*data / snap).round() * snap;
                }
            }
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

        if self.fill_track {
            let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
            let x1 = (ctx.size().width - knob_size) * self.normalize(*data) + knob_size / 2.;
            let background_rect = Rect::from_points(
                (knob_size / 2.0, (knob_size - TRACK_THICKNESS) / 2.),
                (x1, (knob_size + TRACK_THICKNESS) / 2.)
            )
                .to_rounded_rect(2.);

            ctx.fill(background_rect, &env.get(theme::SELECTION_COLOR));
        }

        self.head.draw(ctx, self.normalize(*data), env);
    }
}

impl Widget<(f64, f64)> for RangeSlider {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut (f64, f64), env: &Env) {
        self.max_knob.handle_event(ctx, event, &mut data.1, env, (self.min, self.max));

        if let Some(snap) = self.snap {
            data.1 = (data.1 / snap).round() * snap;
        }
        data.1 = data.1.max(data.0 + self.min_range);

        if !self.max_knob.is_active() {
            self.min_knob.handle_event(ctx, event, &mut data.0, env, (self.min, self.max));

            if let Some(snap) = self.snap {
                data.0 = (data.0 / snap).round() * snap;
            }
            data.0 = data.0.min(data.1 - self.min_range);

            if !self.min_knob.is_active() {
                if let Event::MouseDown(me) = event {
                    let center = (self.min_knob.pos.x + self.max_knob.pos.x) / 2.0;

                    let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
                    let slider_width = ctx.size().width;

                    ctx.set_active(true);
                    if me.pos.x <= center {
                        self.min_knob.x_offset = 0.0;
                        self.min_knob.is_active = true;
                        self.min_knob.pos = me.pos;
                        data.0 = self.calculate_value(me.pos.x, knob_size, slider_width);
                        if let Some(snap) = self.snap {
                            data.0 = (data.0 / snap).round() * snap;
                        }
                        data.0 = data.0.min(data.1 - self.min_range);
                    } else {
                        self.max_knob.x_offset = 0.0;
                        self.max_knob.is_active = true;
                        self.max_knob.pos = me.pos;
                        data.1 = self.calculate_value(me.pos.x, knob_size, slider_width);
                        if let Some(snap) = self.snap {
                            data.1 = (data.1 / snap).round() * snap;
                        }
                        data.1 = data.1.max(data.0 + self.min_range);
                    }
                }
            }

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

        if self.fill_track {
            let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
            let x0 = (ctx.size().width - knob_size) * self.normalize(data.0) + knob_size / 2.;
            let x1 = (ctx.size().width - knob_size) * self.normalize(data.1) + knob_size / 2.;

            let background_rect = Rect::from_points(
                    (x0, (knob_size - TRACK_THICKNESS) / 2.),
                    (x1, (knob_size + TRACK_THICKNESS) / 2.)
                );

            ctx.fill(background_rect, &env.get(theme::SELECTION_COLOR));
        }

        self.min_knob.draw(ctx, self.normalize(data.0), env);
        self.max_knob.draw(ctx, self.normalize(data.1), env);
    }
}

#[derive(Debug, Clone, Default)]
struct SliderHead {
    pos: Point,
    x_offset: f64,
    is_hovered: bool,
    is_active: bool,
}

impl SliderHead {
    fn new() -> Self {
        SliderHead {
            pos: Default::default(),
            x_offset: 0.0,
            is_hovered: false,
            is_active: false
        }
    }
    pub fn handle_event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env, slider_bounds: (f64, f64)) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let slider_width = ctx.size().width;

        match event {
            Event::MouseDown(mouse) => {
                if knob_hit_test(self.pos, knob_size, mouse.pos) {
                    self.is_active = true;
                    ctx.set_active(true);
                    self.x_offset = self.pos.x - mouse.pos.x
                }
            }
            Event::MouseUp(mouse) => {
                if self.is_active {
                    *data = slider_bounds.calculate_value(mouse.pos.x + self.x_offset, knob_size, slider_width);
                    ctx.request_paint();
                    self.is_active = false;
                    ctx.set_active(false);
                }
            }
            Event::MouseMove(mouse) => {
                if self.is_active {
                    *data = slider_bounds.calculate_value(mouse.pos.x + self.x_offset, knob_size, slider_width);
                    ctx.request_paint();
                }
                if ctx.is_hot() {
                    let knob_hover = knob_hit_test(self.pos, knob_size, mouse.pos);
                    if knob_hover != self.is_hovered {
                        self.is_hovered = knob_hover;
                        ctx.request_paint();
                    }
                }
            }
            _ => (),
        }
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    fn draw(&mut self, ctx: &mut PaintCtx, clamped: f64, env: &Env) {
        let rect = ctx.size().to_rect();
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let knob_position = (rect.width() - knob_size) * clamped + knob_size / 2.;
        self.pos = Point::new(knob_position, knob_size / 2.);
        let knob_circle = Circle::new(self.pos, (knob_size - KNOB_STROKE_WIDTH) / 2.);

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

        let knob_gradient = if self.is_active {
            flipped_knob_gradient
        } else {
            normal_knob_gradient
        };

        //Paint the border
        let border_color = if self.is_hovered || self.is_active {
            env.get(theme::FOREGROUND_LIGHT)
        } else {
            env.get(theme::FOREGROUND_DARK)
        };

        ctx.stroke(knob_circle, &border_color, KNOB_STROKE_WIDTH);

        //Actually paint the knob
        ctx.fill(knob_circle, &knob_gradient);
    }
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
    fn build_annotation(&mut self, ctx: &mut PietText, env: &Env) {
        let inner = self.slider.widget();
        let low = (inner.min() / self.step_size).ceil() * self.step_size;
        let high = (inner.max() / self.step_size).floor() * self.step_size;

        self.annotations.clear();

        let mut current = low + 0.0000000001;
        while current <= high + 0.0000000002 {
            let mut text = format!("{}", current);
            while text.len() > 5 || text.ends_with("0") || text.ends_with(".") {
                text.pop();
            }
            let text = ctx.new_text_layout(text)
                .font(FontFamily::SANS_SERIF, 15.0)
                .text_color(env.get(theme::LABEL_COLOR))
                .build()
                .unwrap();

            self.annotations.push(text);

            current += self.step_size;
        }

        self.first = low;

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
            self.build_annotation(ctx.text(), env);
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
            let text_pos = Point::new(
                (knob_position - layout.size().width / 2.0).max(0.0).min(ctx.size().width - layout.size().width),
                large_bottom
            );
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