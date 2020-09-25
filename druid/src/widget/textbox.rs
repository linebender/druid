// Copyright 2018 The Druid Authors.
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

use std::time::Duration;

use crate::widget::prelude::*;
use crate::{
    Application, BoxConstraints, Cursor, Data, Env, FontDescriptor, HotKey, KbKey, KeyOrValue,
    Selector, SysMods, TimerToken,
};

use crate::kurbo::{Affine, Insets, Point, Size};
use crate::theme;

use crate::text::{
    movement, offset_for_delete_backwards, BasicTextInput, EditAction, EditableText, MouseAction,
    Movement, Selection, TextInput, TextLayout,
};

const BORDER_WIDTH: f64 = 1.;
const TEXT_INSETS: Insets = Insets::new(4.0, 2.0, 0.0, 2.0);

// we send ourselves this when we want to reset blink, which must be done in event.
const RESET_BLINK: Selector = Selector::new("druid-builtin.reset-textbox-blink");
const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);

/// A widget that allows user text input.
#[derive(Debug, Clone)]
pub struct TextBox {
    placeholder: String,
    text: TextLayout,
    width: f64,
    hscroll_offset: f64,
    selection: Selection,
    cursor_timer: TimerToken,
    cursor_on: bool,
}

impl TextBox {
    /// Perform an `EditAction`. The payload *must* be an `EditAction`.
    pub const PERFORM_EDIT: Selector<EditAction> =
        Selector::new("druid-builtin.textbox.perform-edit");

    /// Create a new TextBox widget
    pub fn new() -> TextBox {
        let text = TextLayout::new("");
        Self {
            width: 0.0,
            hscroll_offset: 0.,
            text,
            selection: Selection::caret(0),
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
            placeholder: String::new(),
        }
    }

    /// Builder-style method to set the `TextBox`'s placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Builder-style method for setting the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn with_text_size(mut self, size: impl Into<KeyOrValue<f64>>) -> Self {
        self.set_text_size(size);
        self
    }

    /// Builder-style method for setting the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn with_font(mut self, font: impl Into<KeyOrValue<FontDescriptor>>) -> Self {
        self.set_font(font);
        self
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        self.text.set_text_size(size);
    }

    /// Set the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn set_font(&mut self, font: impl Into<KeyOrValue<FontDescriptor>>) {
        self.text.set_font(font);
    }

    /// Insert text at the cursor position.
    /// Replaces selected text if there's a selection.
    fn insert(&mut self, src: &mut String, new: &str) {
        // EditableText's edit method will panic if selection is greater than
        // src length, hence we try to constrain it.
        //
        // This is especially needed when data was modified externally.
        // TODO: perhaps this belongs in update?
        let selection = self.selection.constrain_to(src);

        src.edit(selection.range(), new);
        self.selection = Selection::caret(selection.min() + new.len());
    }

    /// Set the selection to be a caret at the given offset, if that's a valid
    /// codepoint boundary.
    fn caret_to(&mut self, text: &mut String, to: usize) {
        match text.cursor(to) {
            Some(_) => self.selection = Selection::caret(to),
            None => log::error!("You can't move the cursor there."),
        }
    }

    /// Return the active edge of the current selection or cursor.
    // TODO: is this the right name?
    fn cursor(&self) -> usize {
        self.selection.end
    }

    fn do_edit_action(&mut self, edit_action: EditAction, text: &mut String) {
        match edit_action {
            EditAction::Insert(chars) | EditAction::Paste(chars) => self.insert(text, &chars),
            EditAction::Backspace => self.delete_backward(text),
            EditAction::Delete => self.delete_forward(text),
            EditAction::JumpDelete(movement) => {
                self.move_selection(movement, text, true);
                self.delete_forward(text)
            }
            EditAction::JumpBackspace(movement) => {
                self.move_selection(movement, text, true);
                self.delete_backward(text)
            }
            EditAction::Move(movement) => self.move_selection(movement, text, false),
            EditAction::ModifySelection(movement) => self.move_selection(movement, text, true),
            EditAction::SelectAll => self.selection.all(text),
            EditAction::Click(action) => {
                if action.mods.shift() {
                    self.selection.end = action.column;
                } else {
                    self.caret_to(text, action.column);
                }
            }
            EditAction::Drag(action) => self.selection.end = action.column,
        }
    }

    /// Edit a selection using a `Movement`.
    fn move_selection(&mut self, mvmnt: Movement, text: &mut String, modify: bool) {
        // This movement function should ensure all movements are legit.
        // If they aren't, that's a problem with the movement function.
        self.selection = movement(mvmnt, self.selection, text, modify);
    }

    /// Delete to previous grapheme if in caret mode.
    /// Otherwise just delete everything inside the selection.
    fn delete_backward(&mut self, text: &mut String) {
        if self.selection.is_caret() {
            let cursor = self.cursor();
            let new_cursor = offset_for_delete_backwards(&self.selection, text);
            text.edit(new_cursor..cursor, "");
            self.caret_to(text, new_cursor);
        } else {
            text.edit(self.selection.range(), "");
            self.caret_to(text, self.selection.min());
        }
    }

    fn delete_forward(&mut self, text: &mut String) {
        if self.selection.is_caret() {
            // Never touch the characters before the cursor.
            if text.next_grapheme_offset(self.cursor()).is_some() {
                self.move_selection(Movement::Right, text, false);
                self.delete_backward(text);
            }
        } else {
            self.delete_backward(text);
        }
    }

    /// For a given point, returns the corresponding offset (in bytes) of
    /// the grapheme cluster closest to that point.
    fn offset_for_point(&self, point: Point) -> usize {
        // Translating from screenspace to Piet's text layout representation.
        // We need to account for hscroll_offset state and TextBox's padding.
        let translated_point = Point::new(point.x + self.hscroll_offset - TEXT_INSETS.x0, point.y);
        self.text.text_position_for_point(translated_point)
    }

    /// Given an offset (in bytes) of a valid grapheme cluster, return
    /// the corresponding x coordinate of that grapheme on the screen.
    fn x_pos_for_offset(&self, offset: usize) -> f64 {
        self.text.point_for_text_position(offset).x
    }

    /// Calculate a stateful scroll offset
    fn update_hscroll(&mut self) {
        let cursor_x = self.x_pos_for_offset(self.cursor());
        let overall_text_width = self.text.size().width;

        // when advancing the cursor, we want some additional padding
        let padding = TEXT_INSETS.x0 * 2.;
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
        self.cursor_timer = ctx.request_timer(CURSOR_BLINK_DURATION);
    }
}

impl Widget<String> for TextBox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        // Guard against external changes in data?
        self.selection = self.selection.constrain_to(data);
        let mut edit_action = None;

        match event {
            Event::MouseDown(mouse) => {
                ctx.request_focus();
                ctx.set_active(true);

                if !mouse.focus {
                    let cursor_offset = self.offset_for_point(mouse.pos);
                    edit_action = Some(EditAction::Click(MouseAction {
                        row: 0,
                        column: cursor_offset,
                        mods: mouse.mods,
                    }));
                }

                ctx.request_paint();
            }
            Event::MouseMove(mouse) => {
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    let cursor_offset = self.offset_for_point(mouse.pos);
                    edit_action = Some(EditAction::Drag(MouseAction {
                        row: 0,
                        column: cursor_offset,
                        mods: mouse.mods,
                    }));
                    ctx.request_paint();
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    self.cursor_on = !self.cursor_on;
                    ctx.request_paint();
                    self.cursor_timer = ctx.request_timer(CURSOR_BLINK_DURATION);
                }
            }
            Event::Command(ref cmd)
                if ctx.is_focused()
                    && (cmd.is(crate::commands::COPY) || cmd.is(crate::commands::CUT)) =>
            {
                if let Some(text) = data.slice(self.selection.range()) {
                    Application::global().clipboard().put_string(text);
                }
                if !self.selection.is_caret() && cmd.is(crate::commands::CUT) {
                    edit_action = Some(EditAction::Delete);
                }
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(RESET_BLINK) => self.reset_cursor_blink(ctx),
            Event::Command(cmd) if cmd.is(TextBox::PERFORM_EDIT) => {
                let edit = cmd.get_unchecked(TextBox::PERFORM_EDIT);
                self.do_edit_action(edit.to_owned(), data);
            }
            Event::Paste(ref item) => {
                if let Some(string) = item.get_string() {
                    edit_action = Some(EditAction::Paste(string));
                    ctx.request_paint();
                }
            }
            Event::KeyDown(key_event) => {
                let event_handled = match key_event {
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KbKey::Tab).matches(k_e) => {
                        ctx.focus_next();
                        true
                    }
                    k_e if HotKey::new(SysMods::Shift, KbKey::Tab).matches(k_e) => {
                        ctx.focus_prev();
                        true
                    }
                    k_e if HotKey::new(None, KbKey::Enter).matches(k_e) => {
                        // 'enter' should do something, maybe?
                        // but for now we are suppressing it, because we don't want
                        // newlines.
                        true
                    }
                    _ => false,
                };

                if !event_handled {
                    edit_action = BasicTextInput::new().handle_event(key_event);
                }

                ctx.request_paint();
            }
            _ => (),
        }

        if let Some(edit_action) = edit_action {
            let is_select_all = matches!(edit_action, EditAction::SelectAll);

            self.do_edit_action(edit_action, data);
            self.reset_cursor_blink(ctx);
            if data.is_empty() {
                self.text.set_text(self.placeholder.as_str());
                self.selection = Selection::caret(0);
            } else {
                self.text.set_text(data.as_str());
            }
            self.text.rebuild_if_needed(ctx.text(), env);

            if !is_select_all {
                self.update_hscroll();
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &String, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.register_for_focus();
                if data.is_empty() {
                    self.text.set_text(self.placeholder.as_str());
                    self.text.set_text_color(theme::PLACEHOLDER_COLOR);
                } else {
                    self.text.set_text(data.as_str());
                }
                self.text.rebuild_if_needed(ctx.text(), env);
            }
            // an open question: should we be able to schedule timers here?
            LifeCycle::FocusChanged(true) => ctx.submit_command(RESET_BLINK.to(ctx.widget_id())),
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &String, data: &String, env: &Env) {
        let content = if data.is_empty() {
            &self.placeholder
        } else {
            data
        };

        // setting text color rebuilds layout, so don't do it if we don't have to
        if !old_data.same(data) {
            self.selection = self.selection.constrain_to(content);
            self.text.set_text(content.as_str());
            if data.is_empty() {
                self.text.set_text_color(theme::PLACEHOLDER_COLOR);
            } else {
                self.text.set_text_color(theme::LABEL_COLOR);
            }
        }

        self.text.rebuild_if_needed(ctx.text(), env);
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &String,
        env: &Env,
    ) -> Size {
        let width = env.get(theme::WIDE_WIDGET_WIDTH);
        let min_height = env.get(theme::BORDERED_WIDGET_HEIGHT);

        let text_metrics = self.text.size();
        let text_height = text_metrics.height + TEXT_INSETS.y_value();
        let height = text_height.max(min_height);

        let size = bc.constrain((width, height));
        self.width = size.width;
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &String, env: &Env) {
        // Guard against changes in data following `event`
        let content = if data.is_empty() {
            &self.placeholder
        } else {
            data
        };

        self.selection = self.selection.constrain_to(content);

        let height = ctx.size().height;
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let selection_color = env.get(theme::SELECTION_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);

        let is_focused = ctx.is_focused();

        let border_color = if is_focused {
            env.get(theme::PRIMARY_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        // Paint the background
        let clip_rect = Size::new(self.width - BORDER_WIDTH, height)
            .to_rect()
            .inset(-BORDER_WIDTH / 2.0)
            .to_rounded_rect(env.get(theme::TEXTBOX_BORDER_RADIUS));

        ctx.fill(clip_rect, &background_color);

        // Render text, selection, and cursor inside a clip
        ctx.with_save(|rc| {
            rc.clip(clip_rect);

            // Calculate layout
            let text_size = self.text.size();

            // Shift everything inside the clip by the hscroll_offset
            rc.transform(Affine::translate((-self.hscroll_offset, 0.)));

            // Layout, measure, and draw text
            let extra_padding = (height - text_size.height - TEXT_INSETS.y_value()).max(0.) / 2.;
            let text_pos = Point::new(TEXT_INSETS.x0, TEXT_INSETS.y0 + extra_padding);

            // Draw selection rect
            if !self.selection.is_caret() {
                for sel in self.text.rects_for_range(self.selection.range()) {
                    let sel = sel + text_pos.to_vec2();
                    let rounded = sel.to_rounded_rect(1.0);
                    rc.fill(rounded, &selection_color);
                }
            }

            self.text.draw(rc, text_pos);

            // Paint the cursor if focused and there's no selection
            if is_focused && self.cursor_on {
                let line = self.text.cursor_line_for_text_position(self.cursor());
                let line = line + text_pos.to_vec2();
                rc.stroke(line, &cursor_color, 1.);
            }
        });

        // Paint the border
        ctx.stroke(clip_rect, &border_color, BORDER_WIDTH);
    }
}

impl Default for TextBox {
    fn default() -> Self {
        TextBox::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that when data is mutated externally widget
    /// can still be used to insert characters.
    #[test]
    fn data_can_be_changed_externally() {
        let mut widget = TextBox::new();
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
        let mut widget = TextBox::new();
        let mut data = "".to_string();

        widget.insert(&mut data, "\u{0073}\u{006F}\u{0337}\u{0073}");

        widget.delete_backward(&mut data);
        widget.delete_backward(&mut data);

        assert_eq!(data, String::from("\u{0073}\u{006F}"))
    }

    /// Devanagari codepoints are 3 utf-8 code units each.
    #[test]
    fn backspace_devanagari() {
        let mut widget = TextBox::new();
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
