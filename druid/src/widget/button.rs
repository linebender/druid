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
use crate::widget::{Label, LabelText, Painter};
use crate::{
    Affine, BoxConstraints, Data, Env, Event, EventCtx, Insets, LayoutCtx, LifeCycle, LifeCycleCtx,
    LinearGradient, PaintCtx, Point, Rect, RenderContext, Size, UnitPoint, UpdateCtx, Widget,
    WidgetExt,
};

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 4.);

/// A button with a text label.
pub struct Button<T> {
    phantom: std::marker::PhantomData<T>,
}

impl<T: Data> Button<T> {
    /// Create a new button. The closure provided will be called when the button
    /// is clicked.
    pub fn new(
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> impl Widget<T> {
        let painter = Self::painter();
        CenteredLabel::new(text)
            .padding(LABEL_INSETS)
            .background(painter)
            .on_click(action)
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

struct CenteredLabel<T> {
    label: Label<T>,
    label_size: Size,
}

impl<T: Data> CenteredLabel<T> {
    pub fn new(text: impl Into<LabelText<T>>) -> CenteredLabel<T> {
        CenteredLabel {
            label: Label::new(text),
            label_size: Size::ZERO,
        }
    }
}

impl<T: Data> Widget<T> for CenteredLabel<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

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
        bc.debug_check("CenteredLabel");
        let label_bc = bc.loosen();
        self.label_size = self.label.layout(layout_ctx, &label_bc, data, env);
        bc.constrain(self.label_size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let label_offset = (ctx.size().to_vec2() - self.label_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            self.label.paint(ctx, data, env);
        });
    }
}
