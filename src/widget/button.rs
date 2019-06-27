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

use crate::{
    Action, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, WidgetInner,
};

use crate::piet::{Color, FillRule, FontBuilder, Text, TextLayoutBuilder};
use crate::{Piet, Point, RenderContext, Vec2};

const BUTTON_BG_COLOR: Color = Color::rgba32(0x40_40_48_ff);
const BUTTON_HOVER_COLOR: Color = Color::rgba32(0x50_50_58_ff);
const BUTTON_PRESSED_COLOR: Color = Color::rgba32(0x60_60_68_ff);
const LABEL_TEXT_COLOR: Color = Color::rgba32(0xf0_f0_ea_ff);

pub struct Label {
    text: String,
}

pub struct Button {
    label: Label,
}

impl Label {
    /// Discussion question: should this return Label or a wrapped
    /// widget (with WidgetBase)?
    pub fn new(text: impl Into<String>) -> Label {
        Label { text: text.into() }
    }

    fn get_layout(&self, rt: &mut Piet, font_size: f32) -> <Piet as RenderContext>::TextLayout {
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

impl WidgetInner for Label {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _env: &Env) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size);
        let brush = paint_ctx.render_ctx.solid_brush(LABEL_TEXT_COLOR);

        let pos = Vec2::new(0.0, font_size as f64);
        paint_ctx.render_ctx.draw_text(&text_layout, pos, &brush);
    }

    fn layout(&mut self, _layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, _env: &Env) -> Size {
        bc.constrain(Size::new(100.0, 17.0))
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, env: &Env) -> Option<Action> {
        None
    }
}

impl Button {
    pub fn new<S: Into<String>>(label: S) -> Button {
        Button {
            label: Label::new(label),
        }
    }
}

impl WidgetInner for Button {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, env: &Env) {
        let is_active = base_state.is_active();
        let is_hot = base_state.is_hot();
        let bg_color = match (is_active, is_hot) {
            (true, true) => BUTTON_PRESSED_COLOR,
            (false, true) => BUTTON_HOVER_COLOR,
            _ => BUTTON_BG_COLOR,
        };
        let brush = paint_ctx.render_ctx.solid_brush(bg_color);
        let rect = base_state.layout_rect.with_origin(Point::ORIGIN);
        paint_ctx.render_ctx.fill(rect, &brush, FillRule::NonZero);

        self.label.paint(paint_ctx, base_state, env);
    }

    fn layout(&mut self, layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, env: &Env) -> Size {
        self.label.layout(layout_ctx, bc, env)
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, _env: &Env) -> Option<Action> {
        let mut result = None;
        match event {
            Event::Mouse(mouse_event) => {
                if mouse_event.count > 0 {
                    ctx.set_active(true);
                    ctx.invalidate();
                } else {
                    if ctx.is_active() {
                        ctx.set_active(false);
                        ctx.invalidate();
                        if ctx.is_hot() {
                            result = Some(Action::from_str("hit"));
                        }
                    }
                }
            }
            // TODO: don't handle this, handle HotChanged, when that's wired.
            Event::MouseMoved(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
        result
    }
}

/*
use std::any::Any;

use crate::kurbo::{Point, Rect, Size};
use crate::piet::{Color, FillRule, FontBuilder, Piet, RenderContext, Text, TextLayoutBuilder};

use crate::widget::Widget;
use crate::{BoxConstraints, LayoutResult};
use crate::{HandlerCtx, Id, LayoutCtx, MouseEvent, PaintCtx, Ui};

const BUTTON_BG_COLOR: Color = Color::rgba32(0x40_40_48_ff);
const BUTTON_HOVER_COLOR: Color = Color::rgba32(0x50_50_58_ff);
const BUTTON_PRESSED_COLOR: Color = Color::rgba32(0x60_60_68_ff);
const LABEL_TEXT_COLOR: Color = Color::rgba32(0xf0_f0_ea_ff);

/// A text label with no interaction.
pub struct Label {
    label: String,
}

/// A clickable button with a label.
pub struct Button {
    label: Label,
}

impl Label {
    pub fn new<S: Into<String>>(label: S) -> Label {
        Label {
            label: label.into(),
        }
    }

    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }

    fn get_layout(&self, rt: &mut Piet, font_size: f32) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = rt
            .text()
            .new_font_by_name("Segoe UI", font_size)
            .unwrap()
            .build()
            .unwrap();
        rt.text()
            .new_text_layout(&font, &self.label)
            .unwrap()
            .build()
            .unwrap()
    }
}

impl Widget for Label {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size);
        let brush = paint_ctx.render_ctx.solid_brush(LABEL_TEXT_COLOR);

        let pos = Point::new(geom.origin().x, geom.origin().y + font_size as f64);
        paint_ctx
            .render_ctx
            .draw_text(&text_layout, pos.to_vec2(), &brush);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<Size>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        // TODO: measure text properly
        LayoutResult::Size(bc.constrain(Size::new(100.0, 17.0)))
    }

    fn poke(&mut self, payload: &mut dyn Any, ctx: &mut HandlerCtx) -> bool {
        if let Some(string) = payload.downcast_ref::<String>() {
            self.label = string.clone();
            ctx.invalidate();
            true
        } else {
            println!("downcast failed");
            false
        }
    }
}

impl Button {
    pub fn new<S: Into<String>>(label: S) -> Button {
        Button {
            label: Label::new(label),
        }
    }

    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for Button {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        {
            let is_active = paint_ctx.is_active();
            let is_hot = paint_ctx.is_hot();
            let bg_color = match (is_active, is_hot) {
                (true, true) => BUTTON_PRESSED_COLOR,
                (false, true) => BUTTON_HOVER_COLOR,
                _ => BUTTON_BG_COLOR,
            };
            let brush = paint_ctx.render_ctx.solid_brush(bg_color);
            paint_ctx.render_ctx.fill(geom, &brush, FillRule::NonZero);
        }
        self.label.paint(paint_ctx, geom);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<Size>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        self.label.layout(bc, children, size, ctx)
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count > 0 {
            ctx.set_active(true);
        } else {
            ctx.set_active(false);
            if ctx.is_hot() {
                ctx.send_event(true);
            }
        }
        ctx.invalidate();
        true
    }

    fn on_hot_changed(&mut self, _hot: bool, ctx: &mut HandlerCtx) {
        ctx.invalidate();
    }

    fn poke(&mut self, payload: &mut dyn Any, ctx: &mut HandlerCtx) -> bool {
        self.label.poke(payload, ctx)
    }
}
*/
