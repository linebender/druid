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

use crate::piet::{Color, FillRule, FontBuilder, Text, TextLayoutBuilder};
use crate::{Piet, Point, RenderContext};

const BUTTON_BG_COLOR: Color = Color::rgba32(0x40_40_48_ff);
const BUTTON_HOVER_COLOR: Color = Color::rgba32(0x50_50_58_ff);
const BUTTON_PRESSED_COLOR: Color = Color::rgba32(0x60_60_68_ff);
const LABEL_TEXT_COLOR: Color = Color::rgba32(0xf0_f0_ea_ff);

/// A label with static text.
pub struct Label {
    text: String,
}

/// A button with a static label.
pub struct Button {
    label: Label,
}

/// A label with dynamic text.
///
/// The provided closure is called on update, and its return
/// value is used as the text for the label.
pub struct DynLabel<T: Data, F: FnMut(&T, &Env) -> String> {
    label_closure: F,
    phantom: PhantomData<T>,
}

impl Label {
    /// Discussion question: should this return Label or a wrapped
    /// widget (with WidgetPod)?
    pub fn new(text: impl Into<String>) -> Label {
        Label { text: text.into() }
    }

    fn get_layout(&self, rt: &mut Piet, font_size: f64) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = rt
            .text()
            .new_font_by_name("Segoe UI", font_size)
            .unwrap()
            .build()
            .unwrap();
        rt.text()
            .new_text_layout(&font, &self.text)
            .unwrap()
            .build()
            .unwrap()
    }
}

impl<T: Data> Widget<T> for Label {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _base_state: &BaseState, _data: &T, _env: &Env) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx, font_size);
        let brush = paint_ctx.solid_brush(LABEL_TEXT_COLOR);
        paint_ctx.draw_text(&text_layout, (0.0, font_size), &brush);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.constrain((100.0, 17.0))
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

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}
}

impl Button {
    pub fn new(text: impl Into<String>) -> Button {
        Button {
            label: Label::new(text),
        }
    }
}

impl<T: Data> Widget<T> for Button {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        let is_active = base_state.is_active();
        let is_hot = base_state.is_hot();
        let bg_color = match (is_active, is_hot) {
            (true, true) => BUTTON_PRESSED_COLOR,
            (false, true) => BUTTON_HOVER_COLOR,
            _ => BUTTON_BG_COLOR,
        };
        let brush = paint_ctx.solid_brush(bg_color);
        let rect = base_state.layout_rect.with_origin(Point::ORIGIN);
        paint_ctx.fill(rect, &brush, FillRule::NonZero);

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

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}
}

impl<T: Data, F: FnMut(&T, &Env) -> String> DynLabel<T, F> {
    pub fn new(label_closure: F) -> DynLabel<T, F> {
        DynLabel {
            label_closure,
            phantom: Default::default(),
        }
    }

    fn get_layout(
        &mut self,
        rt: &mut Piet,
        font_size: f64,
        data: &T,
        env: &Env,
    ) -> <Piet as RenderContext>::TextLayout {
        let text = (self.label_closure)(data, env);
        // TODO: caching of both the format and the layout
        let font = rt
            .text()
            .new_font_by_name("Segoe UI", font_size)
            .unwrap()
            .build()
            .unwrap();
        rt.text()
            .new_text_layout(&font, &text)
            .unwrap()
            .build()
            .unwrap()
    }
}

impl<T: Data, F: FnMut(&T, &Env) -> String> Widget<T> for DynLabel<T, F> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _base_state: &BaseState, data: &T, env: &Env) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx, font_size, data, env);
        let brush = paint_ctx.solid_brush(LABEL_TEXT_COLOR);
        paint_ctx.draw_text(&text_layout, (0., font_size), &brush);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.constrain(Size::new(100.0, 17.0))
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
