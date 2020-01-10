// Copyright 2018 The xi-editor Authors.
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

//! A button widget.

use crate::kurbo::{Point, RoundedRect, Size};
use crate::theme;
use crate::widget::{Align, Label, LabelText, SizedBox};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LinearGradient, PaintCtx, RenderContext,
    UnitPoint, UpdateCtx, Widget,
};

/// A button with a text label.
pub struct Button<T> {
    label: Label<T>,
    /// A closure that will be invoked when the button is clicked.
    action: Box<dyn Fn(&mut EventCtx, &mut T, &Env)>,
}

impl<T: Data> Button<T> {
    /// Create a new button. The closure provided will be called when the button
    /// is clicked.
    pub fn new(
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> Button<T> {
        Button {
            label: Label::new(text).align(UnitPoint::CENTER),
            action: Box::new(action),
        }
    }

    /// Create a new button with a fixed size.
    pub fn sized(
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
        width: f64,
        height: f64,
    ) -> impl Widget<T> {
        Align::vertical(
            UnitPoint::CENTER,
            SizedBox::new(Button {
                label: Label::new(text).align(UnitPoint::CENTER),
                action: Box::new(action),
            })
            .width(width)
            .height(height),
        )
    }

    /// A function that can be passed to `Button::new`, for buttons with no action.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::Button;
    ///
    /// let button = Button::<u32>::new("hello", Button::noop);
    /// ```
    pub fn noop(_: &mut EventCtx, _: &mut T, _: &Env) {}
}

impl<T: Data> Widget<T> for Button<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.invalidate();
                    if ctx.is_hot() {
                        (self.action)(ctx, data, env);
                    }
                }
            }
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.label.update(ctx, old_data, data, env)
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Button");

        self.label.layout(layout_ctx, bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_active = paint_ctx.is_active();
        let is_hot = paint_ctx.is_hot();

        let rounded_rect =
            RoundedRect::from_origin_size(Point::ORIGIN, paint_ctx.size().to_vec2(), 4.);
        let bg_gradient = if is_active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_LIGHT), env.get(theme::BUTTON_DARK)),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (env.get(theme::BUTTON_DARK), env.get(theme::BUTTON_LIGHT)),
            )
        };

        let border_color = if is_hot {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER)
        };

        paint_ctx.stroke(rounded_rect, &border_color, 2.0);

        paint_ctx.fill(rounded_rect, &bg_gradient);

        self.label.paint(paint_ctx, data, env);
    }
}
