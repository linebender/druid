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

//! A radio button widget.

use crate::kurbo::Circle;
use crate::widget::prelude::*;
use crate::widget::{CrossAxisAlignment, Flex, Label, LabelText};
use crate::{theme, Data, LinearGradient, UnitPoint};

const DEFAULT_RADIO_RADIUS: f64 = 7.0;
const INNER_CIRCLE_RADIUS: f64 = 2.0;
/// A group of radio buttons
#[derive(Debug, Clone)]
pub struct RadioGroup;

impl RadioGroup {
    /// Given a vector of `(label_text, enum_variant)` tuples, create a group of Radio buttons
    pub fn new<T: Data + PartialEq>(
        variants: impl IntoIterator<Item = (impl Into<LabelText<T>> + 'static, T)>,
    ) -> impl Widget<T> {
        let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);
        let mut is_first = true;
        for (label, variant) in variants.into_iter() {
            if !is_first {
                col.add_default_spacer();
            }
            let radio = Radio::new(label, variant);
            col.add_child(radio);
            is_first = false;
        }
        col
    }
}

/// A single radio button
pub struct Radio<T> {
    variant: T,
    child_label: Label<T>,
}

impl<T: Data> Radio<T> {
    /// Create a lone Radio button from label text and an enum variant
    pub fn new(label: impl Into<LabelText<T>>, variant: T) -> Radio<T> {
        Radio {
            variant,
            child_label: Label::new(label),
        }
    }
}

impl<T: Data + PartialEq> Widget<T> for Radio<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        *data = self.variant.clone();
                    }
                    ctx.request_paint();
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child_label.lifecycle(ctx, event, data, env);
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child_label.update(ctx, old_data, data, env);
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Radio");

        let label_size = self.child_label.layout(ctx, &bc, data, env);
        let radio_diam = env.get(theme::BASIC_WIDGET_HEIGHT);
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);

        let desired_size = Size::new(
            label_size.width + radio_diam + x_padding,
            radio_diam.max(label_size.height),
        );
        bc.constrain(desired_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);

        let circle = Circle::new((size / 2., size / 2.), DEFAULT_RADIO_RADIUS);

        // Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        ctx.fill(circle, &background_gradient);

        let border_color = if ctx.is_hot() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(circle, &border_color, 1.);

        // Check if data enum matches our variant
        if *data == self.variant {
            let inner_circle = Circle::new((size / 2., size / 2.), INNER_CIRCLE_RADIUS);

            ctx.fill(inner_circle, &env.get(theme::LABEL_COLOR));
        }

        // Paint the text label
        self.child_label.draw_at(ctx, (size + x_padding, 0.0));
    }
}
