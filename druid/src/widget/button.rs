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
use crate::theme;
use crate::widget::prelude::*;
use crate::widget::{
    Click, Container, ControllerHost, Flex, Label, LabelText, MainAxisAlignment, Padding, Painter,
};
use crate::{Data, Insets, LinearGradient, Point, Rect, RenderContext, UnitPoint, Widget};

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 4.);

/// A button with a text label.
pub struct Button<T> {
    child: Box<dyn Widget<T>>,
}

impl<T: Data> Button<T> {
    /// Create a new button with a text label.
    ///
    /// Use the `.on_click` method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::{Button};
    ///
    /// let button = Button::new("Increment").on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    pub fn new(text: impl Into<LabelText<T>>) -> Button<T> {
        Button::with_child(
            Flex::row()
                .with_child(Label::new(text))
                .main_axis_alignment(MainAxisAlignment::Center),
        )
    }

    /// Create a new button that wraps a child widget.
    ///
    /// The widget will receive padding and a styled background and border. If
    /// you want a clickable widget without the styling, consider just using
    /// `.on_click` from [`WidgetExt`] without the Button.
    ///
    /// [`WidgetExt`]: trait.WidgetExt.html#method.on_click
    pub fn with_child(child: impl Widget<T> + 'static) -> Button<T> {
        let painter = Self::painter();
        Button {
            child: Box::new(Container::new(Padding::new(LABEL_INSETS, child)).background(painter)),
        }
    }

    /// Provide a closure to be called when this button is clicked.
    pub fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }

    fn painter() -> Painter<T> {
        Painter::new(|ctx, _, env| {
            let is_active = ctx.is_active();
            let is_hot = ctx.is_hot();
            let size = ctx.size();
            let border_width = env.get(theme::BUTTON_BORDER_WIDTH);

            let rounded_rect = Rect::from_origin_size(Point::ORIGIN, size)
                .inset(border_width / -2.0)
                .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

            let bg_gradient = if is_active {
                LinearGradient::new(
                    UnitPoint::TOP,
                    UnitPoint::BOTTOM,
                    (env.get(theme::BUTTON_DARK), env.get(theme::BUTTON_LIGHT)),
                )
            } else {
                LinearGradient::new(
                    UnitPoint::TOP,
                    UnitPoint::BOTTOM,
                    (env.get(theme::BUTTON_LIGHT), env.get(theme::BUTTON_DARK)),
                )
            };

            let border_color = if is_hot {
                env.get(theme::BORDER_LIGHT)
            } else {
                env.get(theme::BORDER_DARK)
            };

            ctx.stroke(rounded_rect, &border_color, border_width);

            ctx.fill(rounded_rect, &bg_gradient);
        })
    }
}

impl<T: Data> Widget<T> for Button<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            _ => (),
        }

        self.child.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Button");
        self.child.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env)
    }
}
