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

use crate::environment::{colors, text, Environment, Key};
use crate::widget::Widget;
use crate::{
    BoxConstraints, Geometry, HandlerCtx, Id, KeyEvent, KeyVariant, LayoutCtx, LayoutResult,
    MouseEvent, PaintCtx, Ui,
};

use kurbo::{Line, Rect};
use piet::{FillRule, FontBuilder, RenderContext, Text, TextLayout, TextLayoutBuilder};
use piet_common::Piet;

pub struct TextBox {
    text: String,
    width: f64,
    font: Option<<<Piet<'static> as RenderContext>::Text as Text>::Font>,
}

impl TextBox {
    /// space between the top and bottom bounds of the text and the top and bottom
    /// of the widget.
    pub const V_PADDING: Key<f64> = Key::new("druid.textbox.vertical_padding");
    const DEFAULT_V_PADDING: f64 = 4.0;

    /// The width of the TextBox's stroke
    pub const STROKE_WIDTH: Key<f64> = Key::new("druid.textbox.stroke_width");
    const DEFAULT_STROKE_WIDTH: f64 = 2.0;

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

    fn load_font(&mut self, rt: &mut Piet, font_size: f32, font_name: &str) {
        let font = rt
            .text()
            .new_font_by_name(font_name, font_size)
            .unwrap()
            .build()
            .unwrap();

        self.font = Some(font);
    }

    fn get_layout(
        &mut self,
        rt: &mut Piet,
        font_size: f32,
        font_name: &str,
    ) -> <Piet as RenderContext>::TextLayout {
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
                self.load_font(rt, font_size, font_name);

                //QUESTION this recursion makes me uncomfortable
                //but it solved my borrowing issues!
                return self.get_layout(rt, font_size, font_name);
            }
        };
    }
}

impl Widget for TextBox {
    fn register_defaults(&self, env: &mut Environment) {
        env.theme
            .set(TextBox::V_PADDING, TextBox::DEFAULT_V_PADDING);
        env.theme
            .set(TextBox::STROKE_WIDTH, TextBox::DEFAULT_STROKE_WIDTH);
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let border_color = if paint_ctx.is_focused() {
            paint_ctx.env().theme.get(colors::HIGHLIGHT)
        } else {
            paint_ctx.env().theme.get(colors::ACCESSORY)
        };

        let text_color = paint_ctx.env().theme.get(colors::TEXT);
        let font_size = paint_ctx.env().theme.get(text::TEXT_SIZE) as f32;
        let font_name = paint_ctx.env().theme.get(text::FONT_NAME);
        let stroke_width = paint_ctx.env().theme.get(TextBox::STROKE_WIDTH);

        // Paint the border
        let brush = paint_ctx.render_ctx.solid_brush(border_color).unwrap();

        let (x, y) = geom.pos;
        let (width, height) = geom.size;
        let rect = Rect::new(
            x as f64,
            y as f64,
            x as f64 + width as f64,
            y as f64 + height as f64,
        );

        let clip_rect = Rect::new(
            x as f64,
            y as f64,
            x as f64 + width as f64 - stroke_width as f64,
            y as f64 + height as f64,
        );

        paint_ctx
            .render_ctx
            .stroke(rect, &brush, stroke_width, None);

        // Paint the text
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size, font_name);
        let brush = paint_ctx.render_ctx.solid_brush(text_color).unwrap();

        let pos = (geom.pos.0, geom.pos.1 + font_size);

        let focused = paint_ctx.is_focused();

        //Render text and cursor inside a clip
        paint_ctx
            .render_ctx
            .with_save(|rc| {
                rc.clip(clip_rect, FillRule::NonZero);
                rc.draw_text(&text_layout, pos, &brush);

                // Paint the cursor if focused
                if focused {
                    let brush = rc.solid_brush(border_color).unwrap();

                    let (x, y) = (
                        geom.pos.0 + text_layout.width() as f32 + 2.,
                        geom.pos.1 + 2.,
                    );

                    let line = Line::new(
                        (x as f64, y as f64),
                        (x as f64, y as f64 + font_size as f64),
                    );

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
        _size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        let font_size = ctx.env().theme.get(text::TEXT_SIZE) as f32;
        LayoutResult::Size(bc.constrain((self.width as f32, font_size)))
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count > 0 {
            ctx.set_focused(true);
            ctx.invalidate();
        }
        true
    }
    fn key(&mut self, event: &KeyEvent, ctx: &mut HandlerCtx) -> bool {
        //match on key event
        match event.key {
            KeyVariant::Char(ch) => {
                if ch == '\u{7f}' {
                    self.text.pop();
                } else {
                    self.text.push(ch);
                }
            }
            KeyVariant::Vkey(vk) => match vk {
                VK_BACK => {
                    self.text.pop();
                }
                _ => {}
            },
        }
        // update the text string
        // call invalidate

        ctx.invalidate();
        true
    }
}
