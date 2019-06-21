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

//! A textbox widget.

use crate::widget::Widget;
use crate::{
    BoxConstraints, HandlerCtx, Id, KeyCode, KeyEvent, LayoutCtx, LayoutResult, MouseEvent,
    PaintCtx, Ui,
};

use crate::kurbo::{Line, Rect, Size, Vec2};
use crate::piet::{
    Color, FillRule, FontBuilder, Piet, RenderContext, Text, TextLayout, TextLayoutBuilder,
};

const ACTIVE_BORDER_COLOR: Color = Color::rgb24(0xff_00_00);
const INACTIVE_BORDER_COLOR: Color = Color::rgb24(0x55_55_55);
const TEXT_COLOR: Color = Color::rgb24(0xf0_f0_ea);
const CURSOR_COLOR: Color = Color::WHITE;

const BOX_HEIGHT: f64 = 24.;
const BORDER_WIDTH: f64 = 2.;

pub struct TextBox {
    text: String,
    width: f64,
    font: Option<<<Piet<'static> as RenderContext>::Text as Text>::Font>,
}

impl TextBox {
    pub fn new(default_text: Option<String>, width: f64) -> TextBox {
        TextBox {
            text: default_text.unwrap_or_else(|| String::new()),
            width,
            font: None,
        }
    }
    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }

    fn load_font(&mut self, rt: &mut Piet, font_size: f64) {
        let font = rt
            .text()
            .new_font_by_name("Segoe UI", font_size as f32)
            .unwrap()
            .build()
            .unwrap();

        self.font = Some(font);
    }

    fn get_layout(&mut self, rt: &mut Piet, font_size: f64) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        match &self.font {
            Some(font) => {
                return rt
                    .text()
                    .new_text_layout(&font, &self.text)
                    .unwrap()
                    .build()
                    .unwrap()
            }
            _ => {
                self.load_font(rt, font_size);

                //QUESTION this recursion makes me uncomfortable
                //but it solved my borrowing issues!
                return self.get_layout(rt, font_size);
            }
        };
    }
}

impl Widget for TextBox {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        let border_color = if paint_ctx.is_focused() {
            ACTIVE_BORDER_COLOR
        } else {
            INACTIVE_BORDER_COLOR
        };
        // Paint the border
        let brush = paint_ctx.render_ctx.solid_brush(border_color);
        let clip_rect = geom.with_size(Size::new(geom.width() - BORDER_WIDTH, geom.height()));

        paint_ctx
            .render_ctx
            .stroke(geom, &brush, BORDER_WIDTH, None);

        // Paint the text
        let font_size = BOX_HEIGHT - 4.;
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size);
        let brush = paint_ctx.render_ctx.solid_brush(TEXT_COLOR);

        let height_delta = Vec2::new(0., font_size);
        let pos = geom.origin() + height_delta;

        let focused = paint_ctx.is_focused();

        //Render text and cursor inside a clip
        paint_ctx
            .render_ctx
            .with_save(|rc| {
                rc.clip(clip_rect, FillRule::NonZero);
                rc.draw_text(&text_layout, pos.to_vec2(), &brush);

                // Paint the cursor if focused
                if focused {
                    let brush = rc.solid_brush(CURSOR_COLOR);

                    let xy = geom.origin() + Vec2::new(text_layout.width() as f64 + 2., 2.);
                    let x2y2 = xy + height_delta;
                    let line = Line::new(xy, x2y2);

                    rc.stroke(line, &brush, 1., None);
                }
                Ok(())
            })
            .unwrap();
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<Size>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain(Size::new(self.width, BOX_HEIGHT)))
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count > 0 {
            ctx.set_focused(true);
            ctx.invalidate();
        }
        true
    }

    fn key_down(&mut self, event: &KeyEvent, ctx: &mut HandlerCtx) -> bool {
        match event {
            event if event.key_code == KeyCode::Backspace => {
                self.text.pop();
            }
            event if event.key_code.is_printable() => {
                self.text.push_str(event.text().unwrap_or(""))
            }
            _ => return false,
        }

        ctx.invalidate();
        true
    }
}
