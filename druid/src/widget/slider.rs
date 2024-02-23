// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A slider widget.

use crate::debug_state::DebugState;
use crate::kurbo::{Circle, Line};
use crate::theme::TEXT_COLOR;
use crate::widget::prelude::*;
use crate::widget::Axis;
use crate::{theme, Color, KeyOrValue, LinearGradient, Point, Rect, UnitPoint, Vec2, WidgetPod};
use druid::kurbo::{PathEl, Shape};
use druid::piet::{PietText, PietTextLayout, Text, TextLayout, TextLayoutBuilder};
use tracing::{instrument, trace, warn};

const TRACK_THICKNESS: f64 = 4.0;
const BORDER_WIDTH: f64 = 2.0;
const KNOB_STROKE_WIDTH: f64 = 2.0;

/// A slider, allowing interactive update of a numeric value.
///
/// This slider implements `Widget<f64>`, and works on values clamped
/// in the range `min..max`.
#[derive(Debug, Clone, Default)]
pub struct Slider {
    mapping: SliderValueMapping,
    knob: SliderKnob,
    track_color: Option<KeyOrValue<Color>>,
    knob_style: KnobStyle,
}

/// A range slider, allowing interactive update of two numeric values .
///
/// This slider implements `Widget<(f64, f64)>`, and works on value pairs clamped
/// in the range `min..max`, where the left value is always smaller than the right.
#[derive(Debug, Clone, Default)]
pub struct RangeSlider {
    mapping: SliderValueMapping,
    left_knob: SliderKnob,
    right_knob: SliderKnob,
    track_color: Option<KeyOrValue<Color>>,
    knob_style: KnobStyle,
}

/// A annotated Slider or RangeSlider
pub struct Annotated<T, W: Widget<T>> {
    inner: WidgetPod<T, W>,

    mapping: SliderValueMapping,
    labeled_steps: f64,
    unlabeled_steps: f64,

    labels: Vec<PietTextLayout>,
}

#[derive(Copy, Clone, Debug)]
pub struct SliderValueMapping {
    min: f64,
    max: f64,
    step: Option<f64>,
    axis: Axis,
}

#[derive(Debug, Clone, Default)]
struct SliderKnob {
    hovered: bool,
    active: bool,
    offset: f64,
}

/// The shape of the slider knobs.
#[derive(Debug, Copy, Clone)]
pub enum KnobStyle {
    /// Circle
    Circle,
    /// Wedge
    Wedge,
}

impl Default for KnobStyle {
    fn default() -> Self {
        Self::Circle
    }
}

impl Slider {
    /// Create a new `Slider`.
    pub fn new() -> Slider {
        Default::default()
    }

    /// Builder-style method to set the range covered by this slider.
    ///
    /// The default range is `0.0..1.0`.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.mapping.min = min;
        self.mapping.max = max;
        self
    }

    /// Builder-style method to set the stepping.
    ///
    /// The default step size is `0.0` (smooth).
    pub fn with_step(mut self, step: f64) -> Self {
        if step < 0.0 {
            warn!("bad stepping (must be positive): {}", step);
            return self;
        }
        self.mapping.step = if step > 0.0 {
            Some(step)
        } else {
            // A stepping value of 0.0 would yield an infinite amount of steps.
            // Enforce no stepping instead.
            None
        };
        self
    }

    /// Builder-style method to set the track color.
    ///
    /// The default color is `None`.
    pub fn track_color(mut self, color: impl Into<Option<KeyOrValue<Color>>>) -> Self {
        self.track_color = color.into();
        self
    }

    /// Builder-style method to set the knob style.
    ///
    /// The default is `Circle`.
    pub fn knob_style(mut self, knob_style: KnobStyle) -> Self {
        self.knob_style = knob_style;
        self
    }

    /// Builder-style method to the axis on which the slider moves.
    ///
    /// The default is `Horizontal`.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.mapping.axis = axis;
        self
    }

    /// Returns the Mapping of this Slider.
    pub fn get_mapping(&self) -> SliderValueMapping {
        self.mapping
    }

    /// Builder-style method to create an annotated range slider.
    ///
    pub fn annotated(self, named_steps: f64, unnamed_steps: f64) -> Annotated<f64, Self> {
        let mapping = self.mapping;
        Annotated::new(self, mapping, named_steps, unnamed_steps)
    }
}

impl Widget<f64> for Slider {
    #[instrument(name = "Slider", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64, env: &Env) {
        if !ctx.is_disabled() {
            self.knob
                .handle_input(ctx, event, data, env, self.mapping, self.knob_style);

            ctx.set_active(self.knob.is_active());

            if let Event::MouseDown(me) = event {
                if !self.knob.active {
                    self.knob.activate(0.0);
                    let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
                    *data = self
                        .mapping
                        .calculate_value(me.pos, knob_size, ctx.size(), 0.0);
                    ctx.request_paint();
                    ctx.set_active(true);
                }
            }
        }
    }

    #[instrument(name = "Slider", level = "trace", skip(self, ctx, event, _data, _env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &f64, _env: &Env) {
        match event {
            // checked in LifeCycle::WidgetAdded because logging may not be setup in with_range
            LifeCycle::WidgetAdded => self.mapping.check_range(),
            LifeCycle::DisabledChanged(_) => ctx.request_paint(),
            _ => (),
        }
    }

    #[instrument(
        name = "Slider",
        level = "trace",
        skip(self, ctx, _old_data, _data, _env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &f64, _data: &f64, _env: &Env) {
        ctx.request_paint();
    }

    #[instrument(name = "Slider", level = "trace", skip(self, ctx, bc, _data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &f64, env: &Env) -> Size {
        bc.debug_check("Slider");
        slider_layout(ctx, bc, env, self.mapping)
    }

    #[instrument(name = "Slider", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, env: &Env) {
        paint_slider_background(ctx, 0.0, *data, &self.track_color, self.mapping, env);

        self.knob
            .paint(ctx, *data, env, self.mapping, self.knob_style);
    }

    fn debug_state(&self, data: &f64) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: data.to_string(),
            ..Default::default()
        }
    }
}

impl RangeSlider {
    /// Create a new `RangeSlider`.
    pub fn new() -> RangeSlider {
        Default::default()
    }

    /// Builder-style method to set the range covered by this range slider.
    ///
    /// The default range is `0.0..1.0`.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.mapping.min = min;
        self.mapping.max = max;
        self
    }

    /// Builder-style method to set the stepping.
    ///
    /// The default step size is `0.0` (smooth).
    pub fn with_step(mut self, step: f64) -> Self {
        if step < 0.0 {
            warn!("bad stepping (must be positive): {}", step);
            return self;
        }
        self.mapping.step = if step > 0.0 {
            Some(step)
        } else {
            // A stepping value of 0.0 would yield an infinite amount of steps.
            // Enforce no stepping instead.
            None
        };
        self
    }

    /// Builder-style method to set the track color.
    ///
    /// The default color is `None`.
    pub fn track_color(mut self, color: impl Into<Option<KeyOrValue<Color>>>) -> Self {
        self.track_color = color.into();
        self
    }

    /// Builder-style method to set the knob style.
    ///
    /// The default is `Circle`.
    pub fn knob_style(mut self, knob_style: KnobStyle) -> Self {
        self.knob_style = knob_style;
        self
    }

    /// Builder-style method to set the axis on which the slider moves.
    ///
    /// The default is `Horizontal`.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.mapping.axis = axis;
        self
    }

    /// Returns the Mapping of this Slider.
    pub fn get_mapping(&self) -> SliderValueMapping {
        self.mapping
    }

    /// Builder-style method to create an annotated range slider.
    ///
    pub fn annotated(self, named_steps: f64, unnamed_steps: f64) -> Annotated<(f64, f64), Self> {
        let mapping = self.mapping;
        Annotated::new(self, mapping, named_steps, unnamed_steps)
    }
}

impl Widget<(f64, f64)> for RangeSlider {
    #[instrument(
        name = "RangeSlider",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut (f64, f64), env: &Env) {
        if !ctx.is_disabled() {
            if !self.right_knob.is_active() {
                self.left_knob.handle_input(
                    ctx,
                    event,
                    &mut data.0,
                    env,
                    self.mapping,
                    self.knob_style,
                );
                data.0 = data.0.min(data.1);
                //Ensure that the left knob stays left

                if self.left_knob.is_active() {
                    self.right_knob.deactivate();
                }
            }
            if !self.left_knob.is_active() {
                self.right_knob.handle_input(
                    ctx,
                    event,
                    &mut data.1,
                    env,
                    self.mapping,
                    self.knob_style,
                );
                //Ensure that the right knob stays right
                data.1 = data.1.max(data.0);

                if self.right_knob.is_active() {
                    self.left_knob.deactivate();
                }
            }
            ctx.set_active(self.left_knob.is_active() || self.right_knob.is_active());

            if let Event::MouseDown(me) = event {
                if !self.left_knob.is_active() && !self.right_knob.is_active() {
                    let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
                    let press_value =
                        self.mapping
                            .calculate_value(me.pos, knob_size, ctx.size(), 0.0);

                    if press_value - data.0 < data.1 - press_value {
                        self.left_knob.activate(0.0);
                        data.0 = press_value;
                    } else {
                        self.right_knob.activate(0.0);
                        data.1 = press_value;
                    }
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
        }
    }

    #[instrument(
        name = "RangeSlider",
        level = "trace",
        skip(self, ctx, event, _data, _env)
    )]
    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &(f64, f64),
        _env: &Env,
    ) {
        match event {
            // checked in LifeCycle::WidgetAdded because logging may not be setup in with_range
            LifeCycle::WidgetAdded => self.mapping.check_range(),
            LifeCycle::DisabledChanged(_) => ctx.request_paint(),
            _ => (),
        }
    }

    #[instrument(
        name = "RangeSlider",
        level = "trace",
        skip(self, ctx, _old_data, _data, _env)
    )]
    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: &(f64, f64),
        _data: &(f64, f64),
        _env: &Env,
    ) {
        ctx.request_paint();
    }

    #[instrument(name = "RangeSlider", level = "trace", skip(self, ctx, bc, _data, env))]
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &(f64, f64),
        env: &Env,
    ) -> Size {
        bc.debug_check("Slider");
        slider_layout(ctx, bc, env, self.mapping)
    }

    #[instrument(name = "RangeSlider", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &(f64, f64), env: &Env) {
        paint_slider_background(ctx, data.0, data.1, &self.track_color, self.mapping, env);

        // We paint the left knob at last since it receives events first and therefore behaves like
        // being "on top".
        self.right_knob
            .paint(ctx, data.1, env, self.mapping, self.knob_style);
        self.left_knob
            .paint(ctx, data.0, env, self.mapping, self.knob_style);
    }

    fn debug_state(&self, data: &(f64, f64)) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: format!("{data:?}"),
            ..Default::default()
        }
    }
}

impl<T, W: Widget<T>> Annotated<T, W> {
    pub fn new(
        inner: W,
        mapping: SliderValueMapping,
        labeled_steps: f64,
        unlabeled_steps: f64,
    ) -> Self {
        Annotated {
            inner: WidgetPod::new(inner),

            mapping,
            labeled_steps: labeled_steps.abs(),
            unlabeled_steps: unlabeled_steps.abs(),

            labels: Vec::new(),
        }
    }

    fn sanitise_values(&mut self) {
        let labeled = self.mapping.range() / self.labeled_steps;
        if !labeled.is_finite() || labeled > 100.0 {
            warn!("Annotated: provided labeled interval \"{}\" has too many steps inside the sliders range {}..{}", self.labeled_steps, self.mapping.min, self.mapping.max);
            self.labeled_steps = self.mapping.range() / 5.0;
        }

        let unlabeled = self.mapping.range() / self.unlabeled_steps;
        if !unlabeled.is_finite() || unlabeled > 10000.0 {
            warn!("Annotated: provided unlabeled interval \"{}\" has too many steps inside the sliders range {}..{}", self.unlabeled_steps, self.mapping.min, self.mapping.max);
            self.unlabeled_steps = self.mapping.range() / 20.0;
        }
    }

    fn build_labels(&mut self, text: &mut PietText, text_color: Color) {
        self.labels.clear();

        let mut walk = self.mapping.min;
        while walk < self.mapping.max + f64::EPSILON * 10.0 {
            let layout = text
                .new_text_layout(format!("{walk}"))
                .text_color(text_color)
                .build()
                .unwrap();

            self.labels.push(layout);

            walk += self.labeled_steps;
        }
    }

    fn line_dir(&self) -> Vec2 {
        match self.mapping.axis {
            Axis::Horizontal => Vec2::new(0.0, 1.0),
            Axis::Vertical => Vec2::new(-1.0, 0.0),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Annotated<T, W> {
    #[instrument(name = "Annotated", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    #[instrument(name = "Annotated", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.sanitise_values();
            self.build_labels(ctx.text(), env.get(TEXT_COLOR));
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    #[instrument(
        name = "Annotated",
        level = "trace",
        skip(self, ctx, _old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
        if ctx.env_key_changed(&TEXT_COLOR) {
            self.build_labels(ctx.text(), env.get(TEXT_COLOR));
            ctx.request_paint();
        }
    }

    #[instrument(name = "Annotated", level = "trace", skip(self, bc, ctx, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let label_size = Size::new(40.0, 20.0);

        match self.mapping.axis {
            Axis::Vertical => {
                let child_bc = bc.shrink((label_size.width, 0.0));
                let child_size = self.inner.layout(ctx, &child_bc, data, env);
                self.inner
                    .set_origin(ctx, Point::new(label_size.width, 0.0));

                Size::new(child_size.width + label_size.width, child_size.height)
            }
            Axis::Horizontal => {
                let child_bc = bc.shrink((0.0, label_size.height));
                let child_size = self.inner.layout(ctx, &child_bc, data, env);
                self.inner.set_origin(ctx, Point::ZERO);

                ctx.set_baseline_offset(self.inner.baseline_offset() + label_size.height);
                Size::new(child_size.width, child_size.height + label_size.height)
            }
        }
    }

    #[instrument(name = "Annotated", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let short_stroke = 3.0;
        let long_stroke = 6.0;
        let stroke_offset = 6.0;

        let slider_offset = Point::new(self.inner.layout_rect().x0, self.inner.layout_rect().y0);

        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let slider_size = self.inner.layout_rect().size();

        let text_color = env.get(TEXT_COLOR);

        let mut walk = self.mapping.min;
        while walk < self.mapping.max + f64::EPSILON * 10.0 {
            let center = self
                .mapping
                .get_point(walk, knob_size, slider_size)
                .to_vec2()
                + slider_offset.to_vec2();

            let line = Line::new(
                (center + self.line_dir() * stroke_offset).to_point(),
                (center + self.line_dir() * (stroke_offset + short_stroke)).to_point(),
            );

            ctx.stroke(line, &text_color, 1.0);
            walk += self.unlabeled_steps;
        }

        let mut walk = self.mapping.min;
        let mut labels = self.labels.iter();
        while walk < self.mapping.max + f64::EPSILON * 10.0 {
            let center = self
                .mapping
                .get_point(walk, knob_size, slider_size)
                .to_vec2()
                + slider_offset.to_vec2();

            let line = Line::new(
                (center + self.line_dir() * stroke_offset).to_point(),
                (center + self.line_dir() * (stroke_offset + long_stroke)).to_point(),
            );

            ctx.stroke(line, &text_color, 1.0);

            let label = labels.next().unwrap();
            let origin = match self.mapping.axis {
                Axis::Horizontal => Vec2::new(label.size().width / 2.0, 0.0),
                Axis::Vertical => Vec2::new(label.size().width, label.size().height / 2.0),
            };

            ctx.draw_text(
                label,
                (center + self.line_dir() * (stroke_offset + long_stroke) - origin).to_point(),
            );

            walk += self.labeled_steps;
        }

        self.inner.paint(ctx, data, env);
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: "Annotated".to_string(),
            children: vec![self.inner.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

impl SliderValueMapping {
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            step: None,
            axis: Axis::Horizontal,
        }
    }

    fn calculate_value(
        &self,
        mouse_pos: Point,
        knob_size: f64,
        slider_size: Size,
        offset: f64,
    ) -> f64 {
        // The vertical slider has its lowest value at the bottom.
        let mouse_pos = Point::new(mouse_pos.x, slider_size.height - mouse_pos.y);

        let scalar = (self.axis.major_pos(mouse_pos) - knob_size / 2.)
            / (self.axis.major(slider_size) - knob_size);

        let mut value =
            (self.min + scalar * (self.max - self.min) + offset).clamp(self.min, self.max);

        if let Some(step) = self.step {
            let max_step_value = ((self.max - self.min) / step).floor() * step + self.min;
            if value > max_step_value {
                // edge case: make sure max is reachable
                let left_dist = value - max_step_value;
                let right_dist = self.max - value;
                value = if left_dist < right_dist {
                    max_step_value
                } else {
                    self.max
                };
            } else {
                // snap to discrete intervals
                value = (((value - self.min) / step).round() * step + self.min).min(self.max);
            }
        }
        value
    }

    fn get_point(&self, value: f64, knob_size: f64, widget_size: Size) -> Point {
        let knob_major =
            (self.axis.major(widget_size) - knob_size) * self.normalize(value) + knob_size / 2.;
        let (w, h) = self.axis.pack(knob_major, knob_size / 2.);
        Point::new(w, widget_size.height - h)
    }

    fn normalize(&self, data: f64) -> f64 {
        (data.clamp(self.min, self.max) - self.min) / (self.max - self.min)
    }

    /// check self.min <= self.max, if not swaps the values.
    fn check_range(&mut self) {
        if self.max < self.min {
            warn!(
                "min({}) should be less than max({}), swapping the values",
                self.min, self.max
            );
            std::mem::swap(&mut self.max, &mut self.min);
        }
    }

    /// the distance between min and max
    fn range(&self) -> f64 {
        self.max - self.min
    }
}

impl Default for SliderValueMapping {
    fn default() -> Self {
        SliderValueMapping {
            min: 0.0,
            max: 1.0,
            step: None,
            axis: Axis::Horizontal,
        }
    }
}

impl SliderKnob {
    fn handle_input(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut f64,
        env: &Env,
        mapping: SliderValueMapping,
        knob_style: KnobStyle,
    ) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let slider_size = ctx.size();

        let point_to_val = |point: Point, offset: f64| {
            mapping.calculate_value(point, knob_size, slider_size, offset)
        };

        let hit_test = |val: &mut f64, mouse_pos: Point| {
            let center = mapping.get_point(*val, knob_size, slider_size);
            match knob_style {
                KnobStyle::Circle => center.distance(mouse_pos) < knob_size,
                KnobStyle::Wedge => {
                    (&knob_wedge(center, knob_size, mapping.axis)[..]).winding(mouse_pos) != 0
                }
            }
        };

        match event {
            Event::MouseDown(mouse) => {
                if !ctx.is_disabled() && hit_test(data, mouse.pos) {
                    self.offset = *data - point_to_val(mouse.pos, 0.0);
                    self.active = true;
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse) => {
                if self.active && !ctx.is_disabled() {
                    *data = point_to_val(mouse.pos, self.offset);
                    ctx.request_paint();
                }
                self.active = false;
            }
            Event::MouseMove(mouse) => {
                if !ctx.is_disabled() {
                    if self.active {
                        *data = point_to_val(mouse.pos, self.offset);
                        ctx.request_paint();
                    }
                    if ctx.is_hot() {
                        let knob_hover = hit_test(data, mouse.pos);
                        if knob_hover != self.hovered {
                            self.hovered = knob_hover;
                            ctx.request_paint();
                        }
                    }
                } else {
                    self.active = false
                }
            }
            _ => (),
        }
    }

    fn deactivate(&mut self) {
        self.hovered = false;
        self.active = false;
    }

    fn activate(&mut self, x_offset: f64) {
        self.hovered = true;
        self.active = true;
        self.offset = x_offset;
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn paint(
        &self,
        ctx: &mut PaintCtx,
        value: f64,
        env: &Env,
        settings: SliderValueMapping,
        knob_style: KnobStyle,
    ) {
        let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);

        let knob_gradient = if ctx.is_disabled() {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::DISABLED_FOREGROUND_LIGHT),
                    env.get(theme::DISABLED_FOREGROUND_DARK),
                ),
            )
        } else if self.active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::FOREGROUND_DARK),
                    env.get(theme::FOREGROUND_LIGHT),
                ),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (
                    env.get(theme::FOREGROUND_LIGHT),
                    env.get(theme::FOREGROUND_DARK),
                ),
            )
        };

        //Paint the border
        let border_color = if (self.hovered || self.active) && !ctx.is_disabled() {
            env.get(theme::FOREGROUND_LIGHT)
        } else {
            env.get(theme::FOREGROUND_DARK)
        };

        match knob_style {
            KnobStyle::Circle => {
                let knob_circle = Circle::new(
                    settings.get_point(value, knob_size, ctx.size()),
                    (knob_size - KNOB_STROKE_WIDTH) / 2.,
                );

                ctx.stroke(knob_circle, &border_color, KNOB_STROKE_WIDTH);

                //Actually paint the knob
                ctx.fill(knob_circle, &knob_gradient);
            }
            KnobStyle::Wedge => {
                let center = settings.get_point(value, knob_size, ctx.size());

                let knob_wedge = knob_wedge(center, knob_size, settings.axis);

                ctx.stroke(&knob_wedge[..], &border_color, KNOB_STROKE_WIDTH);

                //Actually paint the knob
                ctx.fill(&knob_wedge[..], &knob_gradient);
            }
        }
    }
}

fn knob_wedge(center: Point, knob_size: f64, axis: Axis) -> [PathEl; 6] {
    let (top, right, left, middle, down) = match axis {
        Axis::Horizontal => (
            Vec2::new(0.0, center.y - knob_size / 2.0),
            Vec2::new(center.x + knob_size / 3.5, 0.0),
            Vec2::new(center.x - knob_size / 3.5, 0.0),
            Vec2::new(0.0, center.y + knob_size / 5.0),
            Vec2::new(center.x, center.y + knob_size / 2.0),
        ),
        Axis::Vertical => (
            Vec2::new(center.x + knob_size / 2.0, 0.0),
            Vec2::new(0.0, center.y + knob_size / 3.5),
            Vec2::new(0.0, center.y - knob_size / 3.5),
            Vec2::new(center.x - knob_size / 5.0, 0.0),
            Vec2::new(center.x - knob_size / 2.0, center.y),
        ),
    };

    [
        PathEl::MoveTo(down.to_point()),
        PathEl::LineTo((right + middle).to_point()),
        PathEl::LineTo((right + top).to_point()),
        PathEl::LineTo((left + top).to_point()),
        PathEl::LineTo((left + middle).to_point()),
        PathEl::ClosePath,
    ]
}

fn paint_slider_background(
    ctx: &mut PaintCtx,
    lower: f64,
    higher: f64,
    track_color: &Option<KeyOrValue<Color>>,
    mapping: SliderValueMapping,
    env: &Env,
) {
    let size = ctx.size();
    let knob_size = env.get(theme::BASIC_WIDGET_HEIGHT);

    //Paint the background
    let background_rect = Rect::from_points(
        mapping.get_point(mapping.min, knob_size, size),
        mapping.get_point(mapping.max, knob_size, size),
    )
    .inset((TRACK_THICKNESS - BORDER_WIDTH) / 2.)
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

    if let Some(color) = track_color {
        let color = color.resolve(env);

        let shape = Rect::from_points(
            mapping.get_point(lower, knob_size, size),
            mapping.get_point(higher, knob_size, size),
        )
        .inset(TRACK_THICKNESS / 2.0)
        .to_rounded_rect(2.);

        ctx.fill(shape, &color);
    }
}

fn slider_layout(
    ctx: &mut LayoutCtx,
    bc: &BoxConstraints,
    env: &Env,
    mapping: SliderValueMapping,
) -> Size {
    let height = env.get(theme::BASIC_WIDGET_HEIGHT);
    let width = env.get(theme::WIDE_WIDGET_WIDTH);
    let size = bc.constrain(mapping.axis.pack(width, height));

    if mapping.axis == Axis::Horizontal {
        let baseline_offset = (height / 2.0) - TRACK_THICKNESS;

        ctx.set_baseline_offset(baseline_offset);
        trace!(
            "Computed layout: size={}, baseline_offset={:?}",
            size,
            baseline_offset
        );
    } else {
        trace!("Computed layout: size={}", size,);
    }

    size
}
