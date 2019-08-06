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

use crate::{
    Action, BaseState, BoxConstraints, Cursor, Env, Event, EventCtx, KeyCode, LayoutCtx, PaintCtx,
    UpdateCtx, Widget,
};

use crate::kurbo::{Affine, Line, Point, RoundedRect, Size, Vec2};
use crate::piet::{
    Color, FillRule, FontBuilder, Piet, RenderContext, Text, TextLayout, TextLayoutBuilder,
};

const BACKGROUND_GREY_LIGHT: Color = Color::rgba32(0x3a_3a_3a_ff);
const BORDER_GREY: Color = Color::rgba32(0x5a_5a_5a_ff);
const PRIMARY_LIGHT: Color = Color::rgba32(0x5c_c4_ff_ff);

const TEXT_COLOR: Color = Color::rgb24(0xf0_f0_ea);
const CURSOR_COLOR: Color = Color::WHITE;

const BOX_HEIGHT: f64 = 24.;
const FONT_SIZE: f64 = 14.0;
const BORDER_WIDTH: f64 = 1.;
const PADDING_TOP: f64 = 5.;
const PADDING_LEFT: f64 = 4.;

#[derive(Debug, Clone)]
pub struct TextBox {
    width: f64,
}

impl TextBox {
    pub fn new(width: f64) -> TextBox {
        TextBox { width }
    }

    fn get_layout(
        &mut self,
        text: &mut <Piet as RenderContext>::Text,
        font_size: f64,
        data: &String,
    ) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = text
            .new_font_by_name("Roboto", font_size)
            .unwrap()
            .build()
            .unwrap();
        text.new_text_layout(&font, data).unwrap().build().unwrap()
    }
}

impl Widget<String> for TextBox {
    fn paint(
        &mut self,
        paint_ctx: &mut PaintCtx,
        base_state: &BaseState,
        data: &String,
        _env: &Env,
    ) {
        let has_focus = base_state.has_focus();

        let border_color = if has_focus {
            PRIMARY_LIGHT
        } else {
            BORDER_GREY
        };

        // Paint the border / background
        let background_brush = paint_ctx.solid_brush(BACKGROUND_GREY_LIGHT);
        let border_brush = paint_ctx.solid_brush(border_color);

        let clip_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                base_state.size().width - BORDER_WIDTH,
                base_state.size().height,
            )
            .to_vec2(),
            2.,
        );

        paint_ctx.fill(clip_rect, &background_brush, FillRule::NonZero);

        paint_ctx.stroke(clip_rect, &border_brush, BORDER_WIDTH, None);

        // Paint the text
        let text = paint_ctx.text();
        let text_layout = self.get_layout(text, FONT_SIZE, data);
        let brush = paint_ctx.solid_brush(TEXT_COLOR);

        let text_height = FONT_SIZE * 0.8;
        let text_pos = Point::new(0.0 + PADDING_LEFT, text_height + PADDING_TOP);

        // Render text and cursor inside a clip
        paint_ctx
            .with_save(|rc| {
                rc.clip(clip_rect, FillRule::NonZero);

                // If overflowing, shift the text
                if text_layout.width() + (PADDING_LEFT * 2.) > self.width {
                    let offset = text_layout.width() - self.width + (PADDING_LEFT * 2.) + 1.;
                    rc.transform(Affine::translate(Vec2::new(-offset, 0.)));
                }
                rc.draw_text(&text_layout, text_pos, &brush);

                // Paint the cursor if focused
                if has_focus {
                    let brush = rc.solid_brush(CURSOR_COLOR);

                    let xy = text_pos + Vec2::new(text_layout.width() + 1., 2. - FONT_SIZE);
                    let x2y2 = xy + Vec2::new(0., FONT_SIZE + 2.);
                    let line = Line::new(xy, x2y2);

                    rc.stroke(line, &brush, 1., None);
                }
                Ok(())
            })
            .unwrap();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        _data: &String,
        _env: &Env,
    ) -> Size {
        Size::new(self.width, BOX_HEIGHT)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut String,
        _env: &Env,
    ) -> Option<Action> {
        match event {
            Event::MouseDown(_) => {
                ctx.request_focus();
                ctx.invalidate();
            }
            Event::MouseMoved(_) => {
                ctx.set_cursor(&Cursor::IBeam);
            }
            Event::KeyDown(key_event) => {
                match key_event {
                    event if event.key_code == KeyCode::Backspace => {
                        data.pop();
                    }
                    event if event.key_code.is_printable() => {
                        data.push_str(event.text().unwrap_or(""));
                    }
                    _ => {}
                }
                ctx.invalidate();
            }
            _ => (),
        }
        None
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: Option<&String>,
        _data: &String,
        _env: &Env,
    ) {
        ctx.invalidate();
    }
}
