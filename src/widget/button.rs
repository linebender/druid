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

use std::marker::PhantomData;

use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size,
    UpdateCtx, Widget,
};

use crate::kurbo::{Rect, RoundedRect};
use crate::piet::{
    FontBuilder, LinearGradient, PietText, PietTextLayout, Text, TextLayout, TextLayoutBuilder,
    UnitPoint,
};

use crate::localization::LocalizedString;
use crate::theme;
use crate::widget::{Align, SizedBox};
use crate::{Point, RenderContext};

/// The text for the label; either a localized or a specific string.
pub enum LabelText<T> {
    Localized(LocalizedString<T>),
    Specific(String),
}

/// A label that displays some text.
pub struct Label<T> {
    text: LabelText<T>,
    align: UnitPoint,
}

/// A button with a text label.
pub struct Button<T> {
    label: Label<T>,
}

/// A label with dynamic text.
///
/// The provided closure is called on update, and its return
/// value is used as the text for the label.
pub struct DynLabel<T: Data, F: FnMut(&T, &Env) -> String> {
    label_closure: F,
    phantom: PhantomData<T>,
}

impl<T: Data> Label<T> {
    /// Discussion question: should this return Label or a wrapped
    /// widget (with WidgetPod)?
    pub fn new(text: impl Into<LabelText<T>>) -> Self {
        Label {
            text: text.into(),
            align: UnitPoint::LEFT,
        }
    }

    pub fn aligned(text: impl Into<LabelText<T>>, align: UnitPoint) -> Self {
        Label {
            text: text.into(),
            align,
        }
    }

    fn get_layout(&self, t: &mut PietText, env: &Env) -> PietTextLayout {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let text = self.text.display_text();
        // TODO: caching of both the format and the layout
        let font = t
            .new_font_by_name(font_name, font_size)
            .unwrap()
            .build()
            .unwrap();
        t.new_text_layout(&font, text).unwrap().build().unwrap()
    }
}

impl<T: Data> Widget<T> for Label<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _data: &T, env: &Env) {
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);

        let text_layout = self.get_layout(paint_ctx.text(), env);

        // Find the origin for the text
        let mut origin = self.align.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                (base_state.size().width - text_layout.width()).max(0.0),
                base_state.size().height + (font_size * 1.2) / 2.,
            ),
        ));

        //Make sure we don't draw the text too low
        origin.y = origin.y.min(base_state.size().height);

        paint_ctx.draw_text(&text_layout, origin, &env.get(theme::LABEL_COLOR));
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        env: &Env,
    ) -> Size {
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let text_layout = self.get_layout(layout_ctx.text(), env);
        // This magical 1.2 constant helps center the text vertically in the rect it's given
        bc.constrain((text_layout.width(), font_size * 1.2))
    }

    fn event(
        &mut self,
        _event: &Event,
        _ctx: &mut EventCtx,
        _data: &mut T,
        _env: &Env,
    ) -> Option<Action> {
        None
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        if self.text.resolve(data, env) {
            ctx.invalidate();
        }
    }
}

impl<T: Data + 'static> Button<T> {
    pub fn new(text: impl Into<LabelText<T>>) -> Button<T> {
        Button {
            label: Label::aligned(text, UnitPoint::CENTER),
        }
    }

    pub fn sized(text: impl Into<LabelText<T>>, width: f64, height: f64) -> impl Widget<T> {
        Align::vertical(
            UnitPoint::CENTER,
            SizedBox::new(Button {
                label: Label::aligned(text, UnitPoint::CENTER),
            })
            .width(width)
            .height(height),
        )
    }
}

impl<T: Data> Widget<T> for Button<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        let is_active = base_state.is_active();
        let is_hot = base_state.is_hot();

        let rounded_rect =
            RoundedRect::from_origin_size(Point::ORIGIN, base_state.size().to_vec2(), 4.);
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

        self.label.paint(paint_ctx, base_state, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        self.label.layout(layout_ctx, bc, data, env)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        _data: &mut T,
        _env: &Env,
    ) -> Option<Action> {
        let mut result = None;
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
                        result = Some(Action::from_str("hit"));
                    }
                }
            }
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
        result
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.label.update(ctx, old_data, data, env)
    }
}

impl<T: Data, F: FnMut(&T, &Env) -> String> DynLabel<T, F> {
    pub fn new(label_closure: F) -> DynLabel<T, F> {
        DynLabel {
            label_closure,
            phantom: Default::default(),
        }
    }

    fn get_layout(&mut self, t: &mut PietText, env: &Env, data: &T) -> PietTextLayout {
        let text = (self.label_closure)(data, env);

        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);

        // TODO: caching of both the format and the layout
        let font = t
            .new_font_by_name(font_name, font_size)
            .unwrap()
            .build()
            .unwrap();
        t.new_text_layout(&font, &text).unwrap().build().unwrap()
    }
}

impl<T: Data, F: FnMut(&T, &Env) -> String> Widget<T> for DynLabel<T, F> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);

        let align = UnitPoint::LEFT;
        let origin = align.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                base_state.size().width,
                base_state.size().height + (font_size * 1.2) / 2.,
            ),
        ));

        let text_layout = self.get_layout(paint_ctx.text(), env, data);
        paint_ctx.draw_text(&text_layout, origin, &env.get(theme::LABEL_COLOR));
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let text_layout = self.get_layout(layout_ctx.text(), env, data);
        // This magical 1.2 constant helps center the text vertically in the rect it's given
        bc.constrain(Size::new(text_layout.width(), font_size * 1.2))
    }

    fn event(
        &mut self,
        _event: &Event,
        _ctx: &mut EventCtx,
        _data: &mut T,
        _env: &Env,
    ) -> Option<Action> {
        None
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {
        ctx.invalidate();
    }
}

impl<T: Data> LabelText<T> {
    /// The text that should be displayed. This ensures that localized
    /// strings are up to date.
    pub fn display_text(&self) -> &str {
        match self {
            LabelText::Specific(s) => s.as_str(),
            LabelText::Localized(s) => s.localized_str(),
        }
    }

    /// Update the localization, if necesasry.
    ///
    /// Returns `true` if the string has changed.
    pub fn resolve(&mut self, data: &T, env: &Env) -> bool {
        match self {
            LabelText::Specific(_) => false,
            LabelText::Localized(s) => s.resolve(data, env),
        }
    }
}

impl<T> From<String> for LabelText<T> {
    fn from(src: String) -> LabelText<T> {
        LabelText::Specific(src)
    }
}

impl<T> From<&str> for LabelText<T> {
    fn from(src: &str) -> LabelText<T> {
        LabelText::Specific(src.to_string())
    }
}

impl<T> From<LocalizedString<T>> for LabelText<T> {
    fn from(src: LocalizedString<T>) -> LabelText<T> {
        LabelText::Localized(src)
    }
}
