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

use std::time::{Duration, Instant};

use crate::{
    Application, BoxConstraints, Cursor, Data, Env, Event, EventCtx, HotKey, KeyCode, LayoutCtx,
    LifeCycle, LifeCycleCtx, PaintCtx, RawMods, Selector, SysMods, TimerToken, UpdateCtx, Widget,
};

use crate::kurbo::{Affine, Line, Point, RoundedRect, Size, Vec2};
use crate::piet::{
    FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
    UnitPoint,
};
use crate::theme;
use crate::widget::Align;

use crate::widget::textbox::{
    movement, offset_for_delete_backwards, EditableText, Movement, Selection,
};

const BORDER_WIDTH: f64 = 1.;
const PADDING_TOP: f64 = 5.;
const PADDING_LEFT: f64 = 4.;

// we send ourselves this when we want to reset blink, which must be done in event.
const RESET_BLINK: Selector = Selector::new("druid-builtin.reset-textbox-blink");

/// A widget that allows user text input.
#[derive(Debug, Clone)]
pub struct TextBox<E: EditableText> {
    placeholder: String,
    width: f64,
    hscroll_offset: f64,
    selection: Selection,
    cursor_timer: TimerToken,
    cursor_on: bool,
    phantom: std::marker::PhantomData<E>,
}

impl<E: 'static + EditableText + Data + std::string::ToString> TextBox<E> {
    /// Create a new TextBox widget
    pub fn new() -> impl Widget<E> {
        Align::vertical(UnitPoint::CENTER, Self::raw())
    }
    /// Create a new TextBox widget with placeholder
    pub fn with_placeholder<T: Into<String>>(placeholder: T) -> impl Widget<E> {
        let mut textbox = Self::raw();
        textbox.placeholder = placeholder.into();
        Align::vertical(UnitPoint::CENTER, textbox)
    }

    /// Create a new TextBox widget with no Align wrapper
    pub fn raw() -> TextBox<E> {
        Self {
            width: 0.0,
            hscroll_offset: 0.,
            selection: Selection::caret(0),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
            placeholder: String::new(),
            phantom: Default::default(),
        }
    }

    fn get_layout(&self, piet_text: &mut PietText, text: &String, env: &Env) -> PietTextLayout {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        // TODO: caching of both the format and the layout
        let font = piet_text
            .new_font_by_name(font_name, font_size)
            .build()
            .unwrap();

        piet_text
            .new_text_layout(&font, &text.to_string())
            .build()
            .unwrap()
    }

    fn insert(&mut self, src: &mut E, new: &str) {
        // TODO: handle incomplete graphemes

        // replace_range will panic if selection is greater than src length hence we try to constrain it.
        // This is especially needed when data was modified externally.
        let selection = self.selection.constrain_to(src);

        src.edit(selection.range(), new);
        self.selection = Selection::caret(selection.min() + new.len());
    }

    fn cursor_to(&mut self, to: usize) {
        // TODO: should we do some codepoint or grapheme checking here?
        self.selection = Selection::caret(to);
    }

    fn cursor(&self) -> usize {
        self.selection.end
    }

    fn move_selection(&mut self, mvmnt: Movement, text: &E, modify: bool) {
        // TODO: should we do some codepoint or grapheme checking here?
        self.selection = movement(mvmnt, self.selection, text, modify);
    }

    /// If it's not a selection, delete to previous grapheme.
    /// If it is a selection, just delete everything inside the selection.
    fn delete_backward(&mut self, text: &mut E) {
        if self.selection.is_caret() {
            let cursor = self.cursor();
            let new_cursor = offset_for_delete_backwards(&self.selection, text);
            text.edit(new_cursor..cursor, "");
            self.cursor_to(new_cursor);
        } else {
            text.edit(self.selection.range(), "");
            self.cursor_to(self.selection.min());
        }
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

    fn reset_cursor_blink(&mut self, ctx: &mut EventCtx) {
        self.cursor_on = true;
        let deadline = Instant::now() + Duration::from_millis(500);
        self.cursor_timer = ctx.request_timer(deadline);
    }
}

impl<E: 'static + EditableText + Data + std::string::ToString> Widget<E> for TextBox<E> {
    #[allow(clippy::cognitive_complexity)]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut E, env: &Env) {
        // Guard against external changes in data
        self.selection = self.selection.constrain_to(data);

        let mut text_layout = self.get_layout(ctx.text(), &data.to_string(), env);
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
                if let Some(text) = data.slice(self.selection.range()) {
                    Application::clipboard().put_string(text);
                }
                if !self.selection.is_caret() && cmd.selector == crate::command::sys::CUT {
                    self.delete_backward(data);
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.selector == RESET_BLINK => self.reset_cursor_blink(ctx),
            Event::Paste(ref item) => {
                if let Some(string) = item.get_string() {
                    self.insert(data, &string);
                    self.reset_cursor_blink(ctx);
                }
            }
            //TODO: move this to a 'handle_key' function, remove the #allow above
            Event::KeyDown(key_event) => {
                match key_event {
                    // Select all (Ctrl+A || Cmd+A)
                    k_e if (HotKey::new(SysMods::Cmd, "a")).matches(k_e) => {
                        self.selection.all(data);
                    }
                    // Jump left (Ctrl+ArrowLeft || Cmd+ArrowLeft)
                    k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowLeft)).matches(k_e)
                        || HotKey::new(None, KeyCode::Home).matches(k_e) =>
                    {
                        self.move_selection(Movement::LeftOfLine, data, false);
                        self.reset_cursor_blink(ctx);
                    }
                    // Jump right (Ctrl+ArrowRight || Cmd+ArrowRight)
                    k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowRight)).matches(k_e)
                        || HotKey::new(None, KeyCode::End).matches(k_e) =>
                    {
                        self.move_selection(Movement::RightOfLine, data, false);
                        self.reset_cursor_blink(ctx);
                    }
                    // Select left (Shift+ArrowLeft)
                    k_e if (HotKey::new(RawMods::Shift, KeyCode::ArrowLeft)).matches(k_e) => {
                        self.move_selection(Movement::Left, data, true);
                    }
                    // Select right (Shift+ArrowRight)
                    k_e if (HotKey::new(RawMods::Shift, KeyCode::ArrowRight)).matches(k_e) => {
                        self.move_selection(Movement::Right, data, true);
                    }
                    // Move left (ArrowLeft)
                    k_e if (HotKey::new(None, KeyCode::ArrowLeft)).matches(k_e) => {
                        self.move_selection(Movement::Left, data, false);
                        self.reset_cursor_blink(ctx);
                    }
                    // Move right (ArrowRight)
                    k_e if (HotKey::new(None, KeyCode::ArrowRight)).matches(k_e) => {
                        self.move_selection(Movement::Right, data, false);
                        self.reset_cursor_blink(ctx);
                    }
                    // Backspace
                    k_e if (HotKey::new(None, KeyCode::Backspace)).matches(k_e) => {
                        self.delete_backward(data);
                        self.reset_cursor_blink(ctx);
                    }
                    // Delete
                    k_e if (HotKey::new(None, KeyCode::Delete)).matches(k_e) => {
                        if self.selection.is_caret() {
                            // Never touch the characters before the cursor.
                            if let Some(_) = data.next_grapheme_offset(self.cursor()) {
                                self.move_selection(Movement::Right, data, false);
                                self.delete_backward(data);
                            }
                        } else {
                            self.delete_backward(data);
                        }
                        self.reset_cursor_blink(ctx);
                    }
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KeyCode::Tab).matches(k_e) => ctx.focus_next(),
                    k_e if HotKey::new(RawMods::Shift, KeyCode::Tab).matches(k_e) => {
                        ctx.focus_prev()
                    }
                    // Actual typing
                    k_e if k_e.key_code.is_printable() => {
                        let incoming_text = k_e.text().unwrap_or("");
                        self.insert(data, incoming_text);
                        self.reset_cursor_blink(ctx);
                    }
                    _ => {}
                }
                text_layout = self.get_layout(ctx.text(), &data.to_string(), env);
                self.update_hscroll(&text_layout);
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &E, _env: &Env) {
        match event {
            LifeCycle::WidgetAdded => ctx.invalidate(),
            LifeCycle::Register => ctx.register_for_focus(),
            // an open question: should we be able to schedule timers here?
            LifeCycle::FocusChanged(true) => ctx.submit_command(RESET_BLINK, ctx.widget_id()),
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &E, _data: &E, _env: &Env) {
        ctx.invalidate();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &E,
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

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &E, env: &Env) {
        // Guard against changes in data following `event`
        let content = if data.is_empty() {
            self.placeholder.clone()
        } else {
            data.to_string()
        };

        self.selection = self.selection.constrain_to(&content);

        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let height = env.get(theme::BORDERED_WIDGET_HEIGHT);
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let selection_color = env.get(theme::SELECTION_COLOR);
        let text_color = env.get(theme::LABEL_COLOR);
        let placeholder_color = env.get(theme::PLACEHOLDER_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);

        let has_focus = paint_ctx.has_focus();

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
                let text_layout = self.get_layout(rc.text(), &content.to_string(), env);

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
                let color = if data.is_empty() {
                    &placeholder_color
                } else {
                    &text_color
                };

                rc.draw_text(&text_layout, text_pos, color);

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

    /// Test backspace on the combo character o̷
    #[test]
    fn backspace_combining() {
        let mut widget = TextBox::raw();
        let mut data = "".to_string();

        widget.insert(&mut data, "\u{0073}\u{006F}\u{0337}\u{0073}");

        widget.delete_backward(&mut data);
        widget.delete_backward(&mut data);

        assert_eq!(data, String::from("\u{0073}\u{006F}"))
    }

    /// Devanagari codepoints are 3 utf-8 code units each.
    #[test]
    fn backspace_devanagari() {
        let mut widget = TextBox::raw();
        let mut data = "".to_string();

        widget.insert(&mut data, "हिन्दी");
        widget.delete_backward(&mut data);
        assert_eq!(data, String::from("हिन्द"));
        widget.delete_backward(&mut data);
        assert_eq!(data, String::from("हिन्"));
        widget.delete_backward(&mut data);
        assert_eq!(data, String::from("हिन"));
        widget.delete_backward(&mut data);
        assert_eq!(data, String::from("हि"));
        widget.delete_backward(&mut data);
        assert_eq!(data, String::from("ह"));
        widget.delete_backward(&mut data);
        assert_eq!(data, String::from(""));
    }
}
