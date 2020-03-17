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
use crate::widget::{Label, LabelText, Painter, WidgetExt};
use crate::{
    Affine, BoxConstraints, Data, Env, Event, EventCtx, Insets, LayoutCtx, LifeCycle, LifeCycleCtx,
    LinearGradient, PaintCtx, Point, Rect, RenderContext, Size, UnitPoint, UpdateCtx, Widget,
};

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

/// A button with a text label.
pub struct Button<T> {
    label: Label<T>,
    label_size: Size,
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
            label: Label::new(text),
            label_size: Size::ZERO,
            action: Box::new(action),
        }
    }

    pub fn click_based_button(
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> impl Widget<T> {
        let button_background = Painter::new(|ctx, _, env| {
            let is_active = ctx.is_active();
            let is_hot = ctx.is_hot();
            let size = ctx.size();

            let rounded_rect = size
                .to_rect()
                .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

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
                env.get(theme::BORDER_DARK)
            };

            ctx.fill(rounded_rect, &bg_gradient);

            ctx.stroke(
                rounded_rect,
                &border_color,
                env.get(theme::BUTTON_BORDER_WIDTH),
            );
        });

        Label::new(text)
            .padding(LABEL_INSETS)
            .background(button_background)
            .rounded(4.) // This should be theme::BUTTON_BORDER_RADIUS
            .on_click(action)
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
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                    if ctx.is_hot() {
                        (self.action)(ctx, data, env);
                    }
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.label.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
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
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let label_bc = bc.shrink(padding).loosen();
        self.label_size = self.label.layout(layout_ctx, &label_bc, data, env);
        // HACK: to make sure we look okay at default sizes when beside a textbox,
        // we make sure we will have at least the same height as the default textbox.
        let min_height = env.get(theme::BORDERED_WIDGET_HEIGHT);

        bc.constrain(Size::new(
            self.label_size.width + padding.width,
            (self.label_size.height + padding.height).max(min_height),
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_active = ctx.is_active();
        let is_hot = ctx.is_hot();
        let size = ctx.size();

        let rounded_rect = Rect::from_origin_size(Point::ORIGIN, size)
            .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

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
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(
            rounded_rect,
            &border_color,
            env.get(theme::BUTTON_BORDER_WIDTH),
        );

        ctx.fill(rounded_rect, &bg_gradient);

        let label_offset = (size.to_vec2() - self.label_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            self.label.paint(ctx, data, env);
        });
    }
}
