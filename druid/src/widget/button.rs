// Copyright 2018 The Druid Authors.
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

use crate::widget::{Click, ControllerHost, Label, LabelText};
use crate::{theme, Affine, Color, Data, Insets, LinearGradient, UnitPoint};
use crate::{widget::prelude::*, KeyOrValue};

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

/// The style values a Button needs to paint itself
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    /// Border width
    pub border_width: KeyOrValue<f64>,
    /// Corner radius
    pub border_radius: KeyOrValue<f64>,
    /// Border color
    pub border_color: KeyOrValue<Color>,
    /// Whether or not to paint the background as a gradient.
    /// If false, the background defaults to background_color_a
    pub background_is_gradient: KeyOrValue<bool>,
    /// First color of gradient, or default color of background
    pub background_color_a: KeyOrValue<Color>,
    /// First color of gradient, or alt color of background
    pub background_color_b: KeyOrValue<Color>,
}

impl ButtonStyle {
    /// Default ButtonStyle
    pub fn new() -> Self {
        Self {
            border_width: theme::BUTTON_BORDER_WIDTH.into(),
            border_radius: theme::BUTTON_BORDER_RADIUS.into(),
            border_color: theme::BORDER_DARK.into(),
            background_is_gradient: true.into(),
            background_color_a: theme::BUTTON_LIGHT.into(),
            background_color_b: theme::BUTTON_DARK.into(),
        }
    }

    /// Default ButtonStyle when button is hovered
    pub fn hot() -> Self {
        let normal = Self::new();

        Self {
            border_color: theme::BORDER_LIGHT.into(),
            ..normal
        }
    }

    /// Default ButtonStyle when button is active
    pub fn active() -> Self {
        let normal = Self::new();

        Self {
            background_color_a: theme::BUTTON_DARK.into(),
            background_color_b: theme::BUTTON_LIGHT.into(),
            ..normal
        }
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// A button with a text label.
pub struct Button<T> {
    label: Label<T>,
    label_size: Size,
    style_normal: ButtonStyle,
    style_hot: ButtonStyle,
    style_active: ButtonStyle,
}

impl<T: Data> Button<T> {
    /// Create a new button with a text label.
    ///
    /// Use the [`.on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::Button;
    ///
    /// let button = Button::new("Increment").on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`.on_click`]: #method.on_click
    pub fn new(text: impl Into<LabelText<T>>) -> Button<T> {
        Button::from_label(Label::new(text))
    }

    /// Create a new button with the provided [`Label`].
    ///
    /// Use the [`.on_click`] method to provide a closure to be called when the
    /// button is clicked.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::Color;
    /// use druid::widget::{Button, Label};
    ///
    /// let button = Button::from_label(Label::new("Increment").with_text_color(Color::grey(0.5))).on_click(|_ctx, data: &mut u32, _env| {
    ///     *data += 1;
    /// });
    /// ```
    ///
    /// [`Label`]: struct.Label.html
    /// [`.on_click`]: #method.on_click
    pub fn from_label(label: Label<T>) -> Button<T> {
        Button {
            label,
            label_size: Size::ZERO,
            style_normal: ButtonStyle::new(),
            style_hot: ButtonStyle::hot(),
            style_active: ButtonStyle::active(),
        }
    }

    /// Customize this button's default style
    pub fn with_style_normal(mut self, style: ButtonStyle) -> Self {
        self.style_normal = style;
        self
    }

    /// Customize this button's style on hover
    pub fn with_style_hot(mut self, style: ButtonStyle) -> Self {
        self.style_hot = style;
        self
    }

    /// Customize this button's style on active
    pub fn with_style_active(mut self, style: ButtonStyle) -> Self {
        self.style_active = style;
        self
    }

    /// Construct a new dynamic button.
    ///
    /// The contents of this button are generated from the data using a closure.
    ///
    /// This is provided as a convenience; a closure can also be passed to [`new`],
    /// but due to limitations of the implementation of that method, the types in
    /// the closure need to be annotated, which is not true for this method.
    ///
    /// # Examples
    ///
    /// The following are equivalent.
    ///
    /// ```
    /// use druid::Env;
    /// use druid::widget::Button;
    /// let button1: Button<u32> = Button::new(|data: &u32, _: &Env| format!("total is {}", data));
    /// let button2: Button<u32> = Button::dynamic(|data, _| format!("total is {}", data));
    /// ```
    ///
    /// [`new`]: #method.new
    pub fn dynamic(text: impl Fn(&T, &Env) -> String + 'static) -> Self {
        let text: LabelText<T> = text.into();
        Button::new(text)
    }

    /// Provide a closure to be called when this button is clicked.
    pub fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }
}

impl<T: Data> Widget<T> for Button<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
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

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Button");
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let label_bc = bc.shrink(padding).loosen();
        self.label_size = self.label.layout(ctx, &label_bc, data, env);
        // HACK: to make sure we look okay at default sizes when beside a textbox,
        // we make sure we will have at least the same height as the default textbox.
        let min_height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let baseline = self.label.baseline_offset();
        ctx.set_baseline_offset(baseline + LABEL_INSETS.y1);

        bc.constrain(Size::new(
            self.label_size.width + padding.width,
            (self.label_size.height + padding.height).max(min_height),
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let is_active = ctx.is_active();
        let is_hot = ctx.is_hot();
        let size = ctx.size();

        // We choose our style up top
        let style = if is_active {
            &self.style_active
        } else if is_hot {
            &self.style_hot
        } else {
            &self.style_normal
        };

        // Now we resolve the specific values we need out of that style
        let stroke_width = style.border_width.resolve(env);
        let stroke_color = style.border_color.resolve(env);
        let is_gradient = style.background_is_gradient.resolve(env);
        let background_a = style.background_color_a.resolve(env);
        let background_b = style.background_color_b.resolve(env);
        let border_radius = style.border_radius.resolve(env);

        let rounded_rect = size
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(border_radius);

        ctx.stroke(rounded_rect, &stroke_color, stroke_width);

        if is_gradient {
            let bg_gradient = LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (background_a, background_b),
            );
            ctx.fill(rounded_rect, &bg_gradient);
        } else {
            ctx.fill(rounded_rect, &background_a);
        }

        let label_offset = (size.to_vec2() - self.label_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            self.label.paint(ctx, data, env);
        });
    }
}
