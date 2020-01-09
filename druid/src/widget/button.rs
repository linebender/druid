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
pub struct Button;

impl Button {
    /// Create a new button with a text label and the default druid styling.
    /// The closure provided will be called when the button is clicked.
    ///
    /// If you want to create a custom button, check out the `Click` widget which
    /// button is based on.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::Button;
    ///
    /// let button = Button::<u32>::new("increment", |_ctx, data, _env| *data += 1);
    /// ```
    pub fn new<T: Data + 'static>(
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> Click<T> {
        Click::new(
            ButtonDecoration::new(Label::new(text).align(UnitPoint::CENTER)),
            action,
        )
    }

    /// Create a new button with a fixed size and a text label.
    pub fn sized<T: Data + 'static>(
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
        width: f64,
        height: f64,
    ) -> impl Widget<T> {
        Align::vertical(
            UnitPoint::CENTER,
            SizedBox::new(Click::new(
                Label::new(text).align(UnitPoint::CENTER),
                action,
            ))
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
    pub fn noop<T: Data + 'static>(_: &mut EventCtx, _: &mut T, _: &Env) {}
}

/// A widget that catches mouse click events
pub struct Click<T: Data> {
    child: Box<dyn Widget<T>>,
    /// A closure that will be invoked when `Click` is clicked.
    action: Box<dyn Fn(&mut EventCtx, &mut T, &Env)>,
}

impl<T: Data + 'static> Click<T> {
    /// Create a new `Click` widget that displays its child.
    ///
    /// Action is a closure that will be invoked when `Click` is clicked.
    ///
    /// `Click` sets its child's context to `is_active` during a click and
    /// `is_hot` when hovered.
    pub fn new(
        child: impl Widget<T> + 'static,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> Click<T> {
        Click {
            child: Box::new(child),
            action: Box::new(action),
        }
    }
}

impl<T: Data> Widget<T> for Click<T> {
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

        self.child.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Click");

        self.child.layout(layout_ctx, &bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(paint_ctx, data, env);
    }
}

struct ButtonDecoration<T: Data> {
    child: Box<dyn Widget<T>>,
}

impl<T: Data> ButtonDecoration<T> {
    pub fn new(child: impl Widget<T> + 'static) -> ButtonDecoration<T> {
        Self {
            child: Box::new(child),
        }
    }
}

impl<T: Data> Widget<T> for ButtonDecoration<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("ButtonDecoration");

        self.child.layout(layout_ctx, &bc, data, env)
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

        //TODO: subtract this added width from the rounded_rect so we don't spill out
        paint_ctx.stroke(rounded_rect, &border_color, 2.0);

        paint_ctx.fill(rounded_rect, &bg_gradient);

        self.child.paint(paint_ctx, data, env);
    }
}
