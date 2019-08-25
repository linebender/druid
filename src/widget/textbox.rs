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
use std::ops::Range;
use std::time::{Duration, Instant};

use crate::{
    Action, BaseState, BoxConstraints, Cursor, Env, Event, EventCtx, HotKey, KeyCode, LayoutCtx,
    PaintCtx, RawMods, SysMods, TimerToken, UpdateCtx, Widget,
};

use crate::kurbo::{Affine, Line, Point, RoundedRect, Size, Vec2};
use crate::piet::{
    Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout,
    TextLayoutBuilder,
};

use crate::unicode_segmentation::GraphemeCursor;

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
    /// Create a selection that begins at start and goes to end.
    /// Like dragging a mouse from start to end.
    pub fn new(start: usize, end: usize) -> Self {
        Selection { start, end }
    }

    /// Create a caret, which is just a selection with the same and start and end.
    pub fn caret(pos: usize) -> Self {
        Selection {
            start: pos,
            end: pos,
        }
    }

    /// If start == end, it's a caret
    pub fn is_caret(self) -> bool {
        self.start == self.end
    }

    /// Return the smallest index (left, in left-to-right languages)
    pub fn min(self) -> usize {
        min(self.start, self.end)
    }

    /// Return the largest index (right, in left-to-right languages)
    pub fn max(self) -> usize {
        max(self.start, self.end)
    }

    /// Return a range from smallest to largest index
    pub fn range(self) -> Range<usize> {
        self.min()..self.max()
    }

    /// Constrain selection to be not greater than input string
    pub fn constrain_to(mut self, s: &str) -> Self {
        let s_len = s.len();
        self.start = min(self.start, s_len);
        self.end = min(self.end, s_len);
        self
    }
}

/// A widget that allows user text input.
#[derive(Debug, Clone)]
pub struct TextBox {
    width: f64,
    hscroll_offset: f64,
    selection: Selection,
    cursor_timer: TimerToken,
    cursor_on: bool,
}

impl TextBox {
    /// Create a new TextBox widget with a width set in pixels
    pub fn new(width: f64) -> TextBox {
        TextBox {
            width,
            hscroll_offset: 0.,
            selection: Selection::caret(0),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
        }
    }

    fn get_layout(&self, text: &mut PietText, font_size: f64, data: &String) -> PietTextLayout {
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

        // replace_range will panic if selection is greater than src length hence we try to constrain it.
        // This is especially needed when data was modified externally.
        let selection = self.selection.constrain_to(src);

        src.replace_range(selection.range(), new);
        self.selection = Selection::caret(selection.min() + new.len());
    }

    fn cursor_to(&mut self, to: usize) {
        self.selection = Selection::caret(to);
    }

    fn cursor(&self) -> usize {
        self.selection.end
    }

    /// Calculate a stateful scroll offset
    fn update_hscroll(&mut self, rc_text: &mut PietText, data: &String) {
        let cursor_x = self.substring_measurement_hack(rc_text, data, 0, self.cursor());
        let overall_text_width = self.substring_measurement_hack(rc_text, data, 0, data.len());

        let padding = PADDING_LEFT * 2.;
        if overall_text_width < self.width {
            // There's no offset if text is smaller than text box
            //
            // [***I*  ]
            // ^
            self.hscroll_offset = 0.;
        } else if cursor_x > self.width + self.hscroll_offset - padding {
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
    }

    // TODO: Grapheme isn't the correct unit for backspace, see:
    // https://github.com/xi-editor/xi-editor/blob/master/rust/core-lib/src/backspace.rs
    fn backspace(&mut self, src: &mut String) {
        if self.selection.is_caret() {
            let cursor = self.cursor();
            let new_cursor = prev_grapheme(&src, cursor);
            src.replace_range(new_cursor..cursor, "");
            self.cursor_to(new_cursor);
        } else {
            src.replace_range(self.selection.range(), "");
            self.cursor_to(self.selection.min());
        }
    }

    // TODO: waiting on druid clipboard support for copy / paste.
    fn copy_text(&self, input: String) {
        eprintln!("COPY: {}", input);
    }

    fn paste_text(&self) -> String {
        eprintln!("PASTE");
        "PASTE".to_string()
    }

    // TODO: do hit testing instead of this substring hack!
    fn substring_measurement_hack(
        &self,
        piet_text: &mut PietText,
        text: &String,
        start: usize,
        end: usize,
    ) -> f64 {
        let mut x: f64 = 0.;

        if let Some(substring) = text.get(start..end) {
            x = self
                .get_layout(piet_text, FONT_SIZE, &substring.to_owned())
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
            Size::new(self.width - BORDER_WIDTH, BOX_HEIGHT).to_vec2(),
            2.,
        );

        paint_ctx.fill(clip_rect, &BACKGROUND_GREY_LIGHT);

        // Render text, selection, and cursor inside a clip
        paint_ctx
            .with_save(|rc| {
                rc.clip(clip_rect);

                // Shift everything inside the clip by the hscroll_offset
                rc.transform(Affine::translate((-self.hscroll_offset, 0.)));

                // Draw selection rect
                if !self.selection.is_caret() {
                    let (left, right) = (self.selection.min(), self.selection.max());

                    let selection_width =
                        self.substring_measurement_hack(rc.text(), data, left, right);

                    let selection_pos = Point::new(
                        self.substring_measurement_hack(rc.text(), data, 0, left) + PADDING_LEFT
                            - 1.,
                        PADDING_TOP - 2.,
                    );
                    let selection_rect = RoundedRect::from_origin_size(
                        selection_pos,
                        Size::new(selection_width + 2., FONT_SIZE + 4.).to_vec2(),
                        1.,
                    );
                    rc.fill(selection_rect, &SELECTION_COLOR);
                }

                // Layout, measure, and draw text
                let text_layout = self.get_layout(rc.text(), FONT_SIZE, data);
                let text_height = FONT_SIZE * 0.8;
                let text_pos = Point::new(0.0 + PADDING_LEFT, text_height + PADDING_TOP);

                rc.draw_text(&text_layout, text_pos, &TEXT_COLOR);

                // Paint the cursor if focused and there's no selection
                if has_focus && self.cursor_on && self.selection.is_caret() {
                    let cursor_x =
                        self.substring_measurement_hack(rc.text(), data, 0, self.cursor());
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
        bc: &BoxConstraints,
        _data: &String,
        _env: &Env,
    ) -> Size {
        bc.constrain((self.width, BOX_HEIGHT))
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
                    k_e if (HotKey::new(SysMods::Cmd, "c")).matches(k_e) => {
                        if let Some(text) = data.get(self.selection.range()) {
                            self.copy_text(text.to_string());
                        }
                    }
                    // Paste (Ctrl+V || Cmd+V)
                    k_e if (HotKey::new(SysMods::Cmd, "v")).matches(k_e) => {
                        let paste_text = self.paste_text();
                        self.insert(data, &paste_text);
                        self.reset_cursor_blink(ctx);
                    }
                    // Select all (Ctrl+A || Cmd+A)
                    k_e if (HotKey::new(SysMods::Cmd, "a")).matches(k_e) => {
                        self.selection = Selection::new(0, data.len());
                    }
                    // Jump left (Ctrl+ArrowLeft || Cmd+ArrowLeft)
                    k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowLeft)).matches(k_e) => {
                        self.cursor_to(0);
                        self.reset_cursor_blink(ctx);
                    }
                    // Jump right (Ctrl+ArrowRight || Cmd+ArrowRight)
                    k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowRight)).matches(k_e) => {
                        self.cursor_to(data.len());
                        self.reset_cursor_blink(ctx);
                    }
                    // Select left (Shift+ArrowLeft)
                    k_e if (HotKey::new(RawMods::Shift, KeyCode::ArrowLeft)).matches(k_e) => {
                        self.selection.end = prev_grapheme(data, self.cursor());
                    }
                    // Select right (Shift+ArrowRight)
                    k_e if (HotKey::new(RawMods::Shift, KeyCode::ArrowRight)).matches(k_e) => {
                        self.selection.end = next_grapheme(data, self.cursor());
                    }
                    // Move left (ArrowLeft)
                    k_e if (HotKey::new(None, KeyCode::ArrowLeft)).matches(k_e) => {
                        if self.selection.is_caret() {
                            self.cursor_to(prev_grapheme(data, self.cursor()));
                        } else {
                            self.cursor_to(self.selection.min());
                        }
                        self.reset_cursor_blink(ctx);
                    }
                    // Move right (ArrowRight)
                    k_e if (HotKey::new(None, KeyCode::ArrowRight)).matches(k_e) => {
                        if self.selection.is_caret() {
                            self.cursor_to(next_grapheme(data, self.cursor()));
                        } else {
                            self.cursor_to(self.selection.max());
                        }
                        self.reset_cursor_blink(ctx);
                    }
                    // Backspace
                    k_e if (HotKey::new(None, KeyCode::Backspace)).matches(k_e) => {
                        self.backspace(data);
                        self.reset_cursor_blink(ctx);
                    }
                    // Actual typing
                    k_e if k_e.key_code.is_printable() => {
                        let incoming_text = k_e.text().unwrap_or("");
                        self.insert(data, incoming_text);
                        self.reset_cursor_blink(ctx);
                    }
                    _ => {}
                }
                self.update_hscroll(ctx.text(), data);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that when data is mutated externally widget
    /// can still be used to insert characters.
    #[test]
    fn data_can_be_changed_externally() {
        let mut widget = TextBox::new(100.);
        let mut data = "".to_string();

        // First insert some chars
        widget.insert(&mut data, "o");
        widget.insert(&mut data, "n");
        widget.insert(&mut data, "e");

        assert_eq!("one", data);
        assert_eq!(3, widget.selection.start);
        assert_eq!(3, widget.selection.end);

        // Modify data externally (e.g data was changed in the parent widget)
        data = "".to_string();

        // Insert again
        widget.insert(&mut data, "a");
    }

}
