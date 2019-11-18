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

//! A radio button widget.

use std::marker::PhantomData;

use crate::kurbo::{Circle, Point, Rect, Size};
use crate::piet::{LinearGradient, RenderContext, UnitPoint};
use crate::theme;
use crate::widget::{Align, Column, Label, LabelText, Padding};
use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
    WidgetPod,
};

/// A group of radio buttons
#[derive(Debug, Clone)]
pub struct RadioGroup<T: Data + PartialEq + 'static> {
    phantom: PhantomData<T>,
}

impl<T: Data + PartialEq + 'static> RadioGroup<T> {
    /// Given a vector of `(label_text, enum_variant)` tuples, create a group of Radio buttons
    pub fn new(
        variants: impl IntoIterator<Item = (impl Into<LabelText<T>> + 'static, T)>,
    ) -> impl Widget<T> {
        let mut col = Column::new();
        for (label, variant) in variants.into_iter() {
            let radio = Radio::new(label, variant);
            col.add_child(Padding::new(5.0, radio), 0.0);
        }
        col
    }
}

/// A single radio button
pub struct Radio<T: Data + PartialEq> {
    variant: T,
    child_label: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data + PartialEq + 'static> Radio<T> {
    /// Create a lone Radio button from label text and an enum variant
    pub fn new(label: impl Into<LabelText<T>>, variant: T) -> impl Widget<T> {
        let radio = Self {
            variant,
            child_label: WidgetPod::new(Label::new(label)).boxed(),
        };
        Align::vertical(UnitPoint::LEFT, radio)
    }
}

impl<T: Data + PartialEq> Widget<T> for Radio<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);

        let circle = Circle::new((size / 2., size / 2.), 7.);

        // Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        paint_ctx.fill(circle, &background_gradient);

        let border_color = if base_state.is_hot() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER)
        };

        paint_ctx.stroke(circle, &border_color, 1.);

        // Check if data enum matches our variant
        if *data == self.variant {
            let inner_circle = Circle::new((size / 2., size / 2.), 2.);

            paint_ctx.fill(inner_circle, &env.get(theme::LABEL_COLOR));
        }

        // Paint the text label
        self.child_label.paint_with_offset(paint_ctx, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Radio");

        let label_size = self.child_label.layout(layout_ctx, &bc, data, env);
        let padding = 5.0;
        let label_x_offset = env.get(theme::BASIC_WIDGET_HEIGHT) + padding;
        let origin = Point::new(label_x_offset, 0.0);

        self.child_label
            .set_layout_rect(Rect::from_origin_size(origin, label_size));

        bc.constrain(Size::new(
            label_x_offset + label_size.width,
            env.get(theme::BASIC_WIDGET_HEIGHT).max(label_size.height),
        ))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        *data = self.variant.clone();
                    }
                    ctx.invalidate();
                }
            }
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {
        ctx.invalidate();
    }
}
