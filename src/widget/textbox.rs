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

use std::cmp::{max, min};
use std::time::{Duration, Instant};

use crate::{
    Action, BaseState, BoxConstraints, Cursor, Env, Event, EventCtx, KeyCode, LayoutCtx, PaintCtx,
    TimerToken, UpdateCtx, Widget,
};

use crate::kurbo::{Affine, Line, Point, RoundedRect, Size, Vec2};
use crate::piet::{Color, FontBuilder, Piet, RenderContext, Text, TextLayout, TextLayoutBuilder};

use druid_shell::clipboard::{ClipboardContext, ClipboardProvider};
use druid_shell::unicode_segmentation::GraphemeCursor;

const BACKGROUND_GREY_LIGHT: Color = Color::rgba8(0x3a, 0x3a, 0x3a, 0xff);
const BORDER_GREY: Color = Color::rgba8(0x5a, 0x5a, 0x5a, 0xff);
const PRIMARY_LIGHT: Color = Color::rgba8(0x5c, 0xc4, 0xff, 0xff);

const SELECTION_COLOR: Color = Color::rgb8(0xf3, 0x00, 0x21);
const TEXT_COLOR: Color = Color::rgb8(0xf0, 0xf0, 0xEA);
const CURSOR_COLOR: Color = Color::WHITE;

const BOX_HEIGHT: f64 = 24.;
const FONT_SIZE: f64 = 14.0;
const BORDER_WIDTH: f64 = 1.;
const PADDING_TOP: f64 = 5.;
const PADDING_LEFT: f64 = 4.;

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    /// The inactive edge of a selection, as a byte offset. When
    /// equal to end, the selection range acts as a caret.
    pub start: usize,

    /// The active edge of a selection, as a byte offset.
    pub end: usize,
}

impl Selection {
    pub fn new(start: usize, end: usize) -> Self {
        Selection { start, end }
    }

    pub fn caret(pos: usize) -> Self {
        Selection {
            start: pos,
            end: pos,
        }
    }

    pub fn min(self) -> usize {
        min(self.start, self.end)
    }

    pub fn max(self) -> usize {
        max(self.start, self.end)
    }

    pub fn is_caret(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone)]
pub struct TextBox {
    width: f64,
    hscroll_offset: f64,
    selection: Selection,
    cursor_timer: TimerToken,
    cursor_on: bool,
}

impl TextBox {
    pub fn new(width: f64) -> TextBox {
        TextBox {
            width,
            hscroll_offset: 0.,
            selection: Selection::caret(3),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
        }
    }

    fn get_layout(
        &self,
        text: &mut <Piet as RenderContext>::Text,
        font_size: f64,
        data: &String,
    ) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = text
            .new_font_by_name("Segoe UI", font_size)
            .unwrap()
            .build()
            .unwrap();
        text.new_text_layout(&font, data).unwrap().build().unwrap()
    }

    fn insert(&mut self, src: &mut String, new: &str) {
        // TODO: handle incomplete graphemes
        let (left, right) = (self.selection.min(), self.selection.max());
        src.replace_range(left..right, new);
        self.selection = Selection::caret(left + new.len());
    }

    fn cursor_to(&mut self, to: usize) {
        self.selection = Selection::caret(to);
    }

    fn cursor(&self) -> usize {
        self.selection.end
    }

    fn backspace(&mut self, src: &mut String) {
        if self.selection.is_caret() {
            let cursor = self.cursor();
            let new_cursor = prev_grapheme(&src, cursor);
            src.replace_range(new_cursor..cursor, "");
            self.cursor_to(new_cursor);
        } else {
            src.replace_range(self.selection.min()..self.selection.max(), "");
            self.cursor_to(self.selection.min());
        }
    }

    fn copy_text(&self, input: String) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.set_contents(input).unwrap();
    }

    fn paste_text(&self) -> String {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.get_contents().unwrap_or("".to_string())
    }

    // TODO: do hit testing instead of this substring hack!
    fn substring_measurement_hack(
        &self,
        rc: &mut Piet,
        text: &String,
        start: usize,
        end: usize,
    ) -> f64 {
        let mut x: f64 = 0.;

        if let Some(substring) = text.get(start..end) {
            x = self
                .get_layout(rc.text(), FONT_SIZE, &substring.to_owned())
                .width();
        }

        x
    }

    fn reset_cursor_blink(&mut self, ctx: &mut EventCtx) {
        self.cursor_on = true;
        let deadline = Instant::now() + Duration::from_millis(500);
        self.cursor_timer = ctx.request_timer(deadline);
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

        // Paint the background
        let clip_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                base_state.size().width - BORDER_WIDTH,
                base_state.size().height,
            )
            .to_vec2(),
            2.,
        );

        paint_ctx.fill(clip_rect, &BACKGROUND_GREY_LIGHT);

        // Render text, selection, and cursor inside a clip
        paint_ctx
            .with_save(|rc| {
                rc.clip(clip_rect);

                // Layout and measure text
                let text = rc.text();
                let text_layout = self.get_layout(text, FONT_SIZE, data);

                let text_height = FONT_SIZE * 0.8;
                let text_pos = Point::new(0.0 + PADDING_LEFT, text_height + PADDING_TOP);

                let cursor_x = self.substring_measurement_hack(rc, data, 0, self.cursor());

                // If overflowing, shift the text
                let padding = PADDING_LEFT * 2.;
                if cursor_x > self.width + self.hscroll_offset - padding {
                    // If cursor goes past right side, bump the offset
                    //       ->
                    // **[****I]****
                    //   ^
                    self.hscroll_offset = cursor_x - self.width + padding;
                } else if cursor_x < self.hscroll_offset {
                    // If cursor goes past left side, match the offset
                    //    <-
                    // **[I****]****
                    //   ^
                    self.hscroll_offset = cursor_x
                }

                rc.transform(Affine::translate(Vec2::new(-self.hscroll_offset, 0.)));

                // Draw selection rect, also shifted
                if !self.selection.is_caret() {
                    let (left, right) = (self.selection.min(), self.selection.max());

                    let selection_width = self.substring_measurement_hack(rc, data, left, right);

                    let selection_pos = Point::new(
                        self.substring_measurement_hack(rc, data, 0, left) + PADDING_LEFT - 1.,
                        PADDING_TOP - 2.,
                    );
                    let selection_rect = RoundedRect::from_origin_size(
                        selection_pos,
                        Size::new(selection_width + 2., FONT_SIZE + 4.).to_vec2(),
                        1.,
                    );
                    rc.fill(selection_rect, &SELECTION_COLOR);
                }

                // Finally draw the text!
                rc.draw_text(&text_layout, text_pos, &TEXT_COLOR);

                // Paint the cursor if focused and there's no selection
                if has_focus && self.cursor_on && self.selection.is_caret() {
                    let xy = text_pos + Vec2::new(cursor_x, 2. - FONT_SIZE);
                    let x2y2 = xy + Vec2::new(0., FONT_SIZE + 2.);
                    let line = Line::new(xy, x2y2);

                    rc.stroke(line, &CURSOR_COLOR, 1.);
                }
                Ok(())
            })
            .unwrap();

        // Paint the border
        paint_ctx.stroke(clip_rect, &border_color, BORDER_WIDTH);
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
                // TODO: hit test and do this for real
                self.cursor_to(self.selection.end);
                ctx.invalidate();
                self.reset_cursor_blink(ctx);
            }
            Event::MouseMoved(_) => {
                ctx.set_cursor(&Cursor::IBeam);
            }
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    self.cursor_on = !self.cursor_on;
                    ctx.invalidate();
                    let deadline = Instant::now() + Duration::from_millis(500);
                    self.cursor_timer = ctx.request_timer(deadline);
                }
            }
            Event::KeyDown(key_event) => {
                match key_event {
                    // Copy (Ctrl+C || Cmd+C)
                    event
                        if (event.mods.meta || event.mods.ctrl)
                            && (event.key_code == KeyCode::KeyC) =>
                    {
                        let (left, right) = (self.selection.min(), self.selection.max());
                        if let Some(text) = data.get(left..right) {
                            self.copy_text(text.to_string());
                        }
                    }
                    // Paste (Ctrl+V || Cmd+V)
                    event
                        if (event.mods.meta || event.mods.ctrl)
                            && (event.key_code == KeyCode::KeyV) =>
                    {
                        let paste_text = self.paste_text();
                        self.insert(data, &paste_text);
                        self.reset_cursor_blink(ctx);
                    }
                    // Select all (Ctrl+A || Cmd+A)
                    event
                        if (event.mods.meta || event.mods.ctrl)
                            && (event.key_code == KeyCode::KeyA) =>
                    {
                        self.selection = Selection::new(0, data.len());
                    }
                    // Jump left (Ctrl+ArrowLeft || Cmd+ArrowLeft)
                    event
                        if (event.mods.meta || event.mods.ctrl)
                            && (event.key_code == KeyCode::ArrowLeft) =>
                    {
                        self.cursor_to(0);
                        self.reset_cursor_blink(ctx);
                    }
                    // Jump right (Ctrl+ArrowRight || Cmd+ArrowRight)
                    event
                        if (event.mods.meta || event.mods.ctrl)
                            && (event.key_code == KeyCode::ArrowRight) =>
                    {
                        self.cursor_to(data.len());
                        self.reset_cursor_blink(ctx);
                    }
                    // Select left (Shift+ArrowLeft)
                    event if event.mods.shift && (event.key_code == KeyCode::ArrowLeft) => {
                        self.selection.end = prev_grapheme(data, self.cursor());
                    }
                    // Select right (Shift+ArrowRight)
                    event if event.mods.shift && (event.key_code == KeyCode::ArrowRight) => {
                        self.selection.end = next_grapheme(data, self.cursor());
                    }
                    // Move left (ArrowLeft)
                    event if event.key_code == KeyCode::ArrowLeft => {
                        if self.selection.is_caret() {
                            self.cursor_to(prev_grapheme(data, self.cursor()));
                        } else {
                            self.cursor_to(self.selection.min());
                        }
                        self.reset_cursor_blink(ctx);
                    }
                    // Move right (ArrowRight)
                    event if event.key_code == KeyCode::ArrowRight => {
                        if self.selection.is_caret() {
                            self.cursor_to(next_grapheme(data, self.cursor()));
                        } else {
                            self.cursor_to(self.selection.max());
                        }
                        self.reset_cursor_blink(ctx);
                    }
                    // Backspace
                    event if event.key_code == KeyCode::Backspace => {
                        self.backspace(data);
                        self.reset_cursor_blink(ctx);
                    }
                    // Actual typing
                    event if event.key_code.is_printable() => {
                        let incoming_text = event.text().unwrap_or("");
                        self.insert(data, incoming_text);
                        self.reset_cursor_blink(ctx);
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

fn next_grapheme(src: &str, from: usize) -> usize {
    let mut c = GraphemeCursor::new(from, src.len(), true);
    let next_boundary = c.next_boundary(src, 0).unwrap();
    if let Some(next) = next_boundary {
        next
    } else {
        src.len()
    }
}

fn prev_grapheme(src: &str, from: usize) -> usize {
    let mut c = GraphemeCursor::new(from, src.len(), true);
    let prev_boundary = c.prev_boundary(src, 0).unwrap();
    if let Some(prev) = prev_boundary {
        prev
    } else {
        0
    }
}
