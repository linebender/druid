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
use unicode_segmentation::GraphemeCursor;

use crate::{
    Application, BaseState, BoxConstraints, Cursor, Env, Event, EventCtx, HotKey, KeyCode,
    LayoutCtx, PaintCtx, RawMods, SysMods, TimerToken, UpdateCtx, Widget,
};

use crate::kurbo::{Affine, Line, Point, RoundedRect, Size, Vec2};
use crate::piet::{
    FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
    UnitPoint,
};
use crate::theme;
use crate::widget::Align;

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
    /// Create a new TextBox widget
    pub fn new() -> impl Widget<String> {
        Align::vertical(UnitPoint::CENTER, Self::raw())
    }

    /// Create a new TextBox widget with no Align wrapper
    pub fn raw() -> TextBox {
        Self {
            width: 0.0,
            hscroll_offset: 0.,
            selection: Selection::caret(0),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
        }
    }

    fn get_layout(&self, piet_text: &mut PietText, data: &str, env: &Env) -> PietTextLayout {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        // TODO: caching of both the format and the layout
        let font = piet_text
            .new_font_by_name(font_name, font_size)
            .build()
            .unwrap();

        piet_text.new_text_layout(&font, data).build().unwrap()
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

    /// For a given point, returns the corresponding offset (in bytes) of
    /// the grapheme cluster closest to that point.
    fn offset_for_point(&self, point: Point, layout: &PietTextLayout) -> usize {
        // Translating from screenspace to Piet's text layout representation.
        // We need to account for hscroll_offset state and TextBox's padding.
        let translated_point = Point::new(point.x + self.hscroll_offset - PADDING_LEFT, point.y);
        let hit_test = layout.hit_test_point(translated_point);
        hit_test.metrics.text_position
    }

    /// Given an offset (in bytes) of a valid grapheme cluster, return
    /// the corresponding x coordinate of that grapheme on the screen.
    fn x_for_offset(&self, layout: &PietTextLayout, offset: usize) -> f64 {
        if let Some(position) = layout.hit_test_text_position(offset) {
            position.point.x
        } else {
            //TODO: what is the correct fallback here?
            0.0
        }
    }

    /// Calculate a stateful scroll offset
    fn update_hscroll(&mut self, layout: &PietTextLayout) {
        let cursor_x = self.x_for_offset(layout, self.cursor());
        let overall_text_width = layout.width();

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
        env: &Env,
    ) {
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let selection_color = env.get(theme::SELECTION_COLOR);
        let text_color = env.get(theme::LABEL_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);

        let has_focus = base_state.has_focus();

        let border_color = if has_focus {
            env.get(theme::PRIMARY_LIGHT)
        } else {
            env.get(theme::BORDER)
        };

        // Paint the background
        let clip_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            Size::new(self.width - BORDER_WIDTH, height).to_vec2(),
            2.,
        );

        paint_ctx.fill(clip_rect, &background_color);

        // Render text, selection, and cursor inside a clip
        paint_ctx
            .with_save(|rc| {
                rc.clip(clip_rect);

                // Calculate layout
                let text_layout = self.get_layout(rc.text(), data, env);

                // Shift everything inside the clip by the hscroll_offset
                rc.transform(Affine::translate((-self.hscroll_offset, 0.)));

                // Draw selection rect
                if !self.selection.is_caret() {
                    let (left, right) = (self.selection.min(), self.selection.max());
                    let left_offset = self.x_for_offset(&text_layout, left);
                    let right_offset = self.x_for_offset(&text_layout, right);

                    let selection_width = right_offset - left_offset;

                    let selection_pos =
                        Point::new(left_offset + PADDING_LEFT - 1., PADDING_TOP - 2.);
                    let selection_rect = RoundedRect::from_origin_size(
                        selection_pos,
                        Size::new(selection_width + 2., font_size + 4.).to_vec2(),
                        1.,
                    );
                    rc.fill(selection_rect, &selection_color);
                }

                // Layout, measure, and draw text
                let text_height = font_size * 0.8;
                let text_pos = Point::new(0.0 + PADDING_LEFT, text_height + PADDING_TOP);

                rc.draw_text(&text_layout, text_pos, &text_color);

                // Paint the cursor if focused and there's no selection
                if has_focus && self.cursor_on && self.selection.is_caret() {
                    let cursor_x = self.x_for_offset(&text_layout, self.cursor());
                    let xy = text_pos + Vec2::new(cursor_x, 2. - font_size);
                    let x2y2 = xy + Vec2::new(0., font_size + 2.);
                    let line = Line::new(xy, x2y2);

                    rc.stroke(line, &cursor_color, 1.);
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
        env: &Env,
    ) -> Size {
        let default_width = 100.0;

        if bc.is_width_bounded() {
            self.width = bc.max().width;
        } else {
            self.width = default_width;
        }

        bc.constrain((self.width, env.get(theme::BORDERED_WIDGET_HEIGHT)))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        let mut text_layout = self.get_layout(ctx.text(), data, env);
        match event {
            Event::MouseDown(mouse) => {
                ctx.request_focus();
                ctx.set_active(true);
                let cursor_off = self.offset_for_point(mouse.pos, &text_layout);
                if mouse.mods.shift {
                    self.selection.end = cursor_off;
                } else {
                    self.cursor_to(cursor_off);
                }
                ctx.invalidate();
                self.reset_cursor_blink(ctx);
            }
            Event::MouseMoved(mouse) => {
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    self.selection.end = self.offset_for_point(mouse.pos, &text_layout);
                    ctx.invalidate();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.invalidate();
                }
            }
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    self.cursor_on = !self.cursor_on;
                    ctx.invalidate();
                    let deadline = Instant::now() + Duration::from_millis(500);
                    self.cursor_timer = ctx.request_timer(deadline);
                }
            }
            Event::Command(ref cmd)
                if ctx.has_focus()
                    && (cmd.selector == crate::commands::COPY
                        || cmd.selector == crate::commands::CUT) =>
            {
                if let Some(text) = data.get(self.selection.range()) {
                    Application::clipboard().put_string(text);
                }
                if !self.selection.is_caret() && cmd.selector == crate::commands::CUT {
                    self.backspace(data);
                }
                ctx.set_handled();
            }
            Event::Paste(ref item) => {
                if let Some(string) = item.get_string() {
                    self.insert(data, &string);
                    self.reset_cursor_blink(ctx);
                }
            }
            Event::KeyDown(key_event) => {
                match key_event {
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
                    // Delete
                    k_e if (HotKey::new(None, KeyCode::Delete)).matches(k_e) => {
                        if self.selection.is_caret() {
                            // Never touch the characters before the cursor.
                            if next_grapheme_exists(data, self.cursor()) {
                                self.cursor_to(next_grapheme(data, self.cursor()));
                                self.backspace(data);
                            }
                        } else {
                            self.backspace(data);
                        }
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
                text_layout = self.get_layout(ctx.text(), data, env);
                self.update_hscroll(&text_layout);
                ctx.invalidate();
            }
            _ => (),
        }
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

/// Gets the next character from the given index.
fn next_grapheme(src: &str, from: usize) -> usize {
    let mut c = GraphemeCursor::new(from, src.len(), true);
    let next_boundary = c.next_boundary(src, 0).unwrap();
    if let Some(next) = next_boundary {
        next
    } else {
        src.len()
    }
}

/// Checks if there is a next character from the given index.
fn next_grapheme_exists(src: &str, from: usize) -> bool {
    let mut c = GraphemeCursor::new(from, src.len(), true);
    let next_boundary = c.next_boundary(src, 0).unwrap();
    if let Some(_next) = next_boundary {
        true
    } else {
        false
    }
}

/// Gets the previous character from the given index.
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
        let mut widget = TextBox::raw();
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
