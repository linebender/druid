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

use crate::kurbo::Vec2;
use crate::text::{
    BasicTextInput, EditAction, EditableText, Editor, LayoutMetrics, TextInput, TextLayout,
    TextStorage,
};
use crate::widget::prelude::*;
use crate::{
    theme, Affine, Color, Cursor, FontDescriptor, HotKey, KbKey, KeyOrValue, Point, Selector,
    SysMods, TextAlignment, TimerToken,
};

const MAC_OR_LINUX: bool = cfg!(any(target_os = "macos", target_os = "linux"));
const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);

/// A widget that allows user text input.
#[derive(Debug, Clone)]
pub struct TextBox<T> {
    placeholder: TextLayout<String>,
    editor: Editor<T>,
    // this can be Box<dyn TextInput> in the future
    input_handler: BasicTextInput,
    hscroll_offset: f64,
    // in cases like SelectAll, we don't adjust the viewport after an event.
    suppress_adjust_hscroll: bool,
    cursor_timer: TimerToken,
    cursor_on: bool,
    multiline: bool,
    alignment: TextAlignment,
    alignment_offset: f64,
    text_pos: Point,
    /// true if a click event caused us to gain focus.
    ///
    /// On macOS, if focus happens via click then we set the selection based
    /// on the click position; if focus happens automatically (e.g. on tab)
    /// then we select our entire contents.
    was_focused_from_click: bool,
}

impl TextBox<()> {
    /// Perform an `EditAction`.
    pub const PERFORM_EDIT: Selector<EditAction> =
        Selector::new("druid-builtin.textbox.perform-edit");
}

impl<T> TextBox<T> {
    /// Create a new TextBox widget.
    pub fn new() -> Self {
        let mut placeholder = TextLayout::from_text("");
        placeholder.set_text_color(theme::PLACEHOLDER_COLOR);
        Self {
            editor: Editor::new(),
            input_handler: BasicTextInput::default(),
            hscroll_offset: 0.,
            suppress_adjust_hscroll: false,
            cursor_timer: TimerToken::INVALID,
            cursor_on: false,
            placeholder,
            multiline: false,
            alignment: TextAlignment::Start,
            alignment_offset: 0.0,
            text_pos: Point::ZERO,
            was_focused_from_click: false,
        }
    }

    /// Create a new multi-line `TextBox`.
    pub fn multiline() -> Self {
        let mut this = TextBox::new();
        this.editor.set_multiline(true);
        this.multiline = true;
        this
    }

    /// Builder-style method to set the `TextBox`'s placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder.set_text(placeholder.into());
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

    /// Builder-style method to set the [`TextAlignment`].
    ///
    /// This is only relevant when the `TextBox` is *not* [`multiline`],
    /// in which case it determines how the text is positioned inside the
    /// `TextBox` when it does not fill the available space.
    ///
    /// # Note:
    ///
    /// This does not behave exactly like [`TextAlignment`] does when used
    /// with label; in particular this does not account for reading direction.
    /// This means that `TextAlignment::Start` (the default) always means
    /// *left aligned*, and `TextAlignment::End` always means *right aligned*.
    ///
    /// This should be considered a bug, but it will not be fixed until proper
    /// BiDi support is implemented.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    /// [`multiline`]: #method.multiline
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.set_text_alignment(alignment);
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

    /// Builder-style method for setting the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.set_text_color(color);
        self
    }

    /// Set the `TextBox`'s placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder.set_text(placeholder.into());
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: ../struct.Key.html
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        let size = size.into();
        self.editor.layout_mut().set_text_size(size.clone());
        self.placeholder.set_text_size(size);
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
        let font = font.into();
        self.editor.layout_mut().set_font(font.clone());
        self.placeholder.set_font(font);
    }

    /// Set the [`TextAlignment`] for this `TextBox``.
    ///
    /// This is only relevant when the `TextBox` is *not* [`multiline`],
    /// in which case it determines how the text is positioned inside the
    /// `TextBox` when it does not fill the available space.
    ///
    /// # Note:
    ///
    /// This does not behave exactly like [`TextAlignment`] does when used
    /// with label; in particular this does not account for reading direction.
    /// This means that `TextAlignment::Start` (the default) always means
    /// *left aligned*, and `TextAlignment::End` always means *right aligned*.
    ///
    /// This should be considered a bug, but it will not be fixed until proper
    /// BiDi support is implemented.
    ///
    /// [`TextAlignment`]: enum.TextAlignment.html
    /// [`multiline`]: #method.multiline
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.alignment = alignment;
    }

    /// Set the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.editor.layout_mut().set_text_color(color);
    }

    /// Return the [`Editor`] used by this `TextBox`.
    ///
    /// This is only needed in advanced cases, such as if you want to customize
    /// the drawing of the text.
    pub fn editor(&self) -> &Editor<T> {
        &self.editor
    }

    /// The point, relative to the origin, where this text box draws its
    /// [`TextLayout`].
    ///
    /// This is exposed in case the user wants to do additional drawing based
    /// on properties of the text.
    ///
    /// This is not valid until `layout` has been called.
    pub fn text_position(&self) -> Point {
        self.text_pos
    }
}

impl<T: TextStorage + EditableText> TextBox<T> {
    /// Calculate a stateful scroll offset
    fn update_hscroll(&mut self, self_width: f64, env: &Env) {
        let cursor_x = self.editor.cursor_line().p0.x;
        // if the text ends in trailing whitespace, that space is not included
        // in its reported width, but we need to include it for these calculations.
        // see https://github.com/linebender/druid/issues/1430
        let overall_text_width = self.editor.layout().size().width.max(cursor_x);
        let text_insets = env.get(theme::TEXTBOX_INSETS);

        //// when advancing the cursor, we want some additional padding
        if overall_text_width < self_width - text_insets.x_value() {
            // There's no offset if text is smaller than text box
            //
            // [***I*  ]
            // ^
            self.hscroll_offset = 0.;
        } else if cursor_x > self_width - text_insets.x_value() + self.hscroll_offset {
            // If cursor goes past right side, bump the offset
            //       ->
            // **[****I]****
            //   ^
            self.hscroll_offset = cursor_x - self_width + text_insets.x_value();
        } else if cursor_x < self.hscroll_offset {
            // If cursor goes past left side, match the offset
            //    <-
            // **[I****]****
            //   ^
            self.hscroll_offset = cursor_x;
        } else if self.hscroll_offset > overall_text_width - self_width + text_insets.x_value() {
            // If the text is getting shorter, keep as small offset as possible
            //        <-
            // **[****I]
            //   ^
            self.hscroll_offset = overall_text_width - self_width + text_insets.x_value();
        }
    }

    fn reset_cursor_blink(&mut self, token: TimerToken) {
        self.cursor_on = true;
        self.cursor_timer = token;
    }

    // on macos we only draw the cursor if the selection is non-caret
    #[cfg(target_os = "macos")]
    fn should_draw_cursor(&self) -> bool {
        self.cursor_on && self.editor.selection().is_caret()
    }

    #[cfg(not(target_os = "macos"))]
    fn should_draw_cursor(&self) -> bool {
        self.cursor_on
    }

    fn update_alignment_adjustment(&mut self, available_width: f64, metrics: &LayoutMetrics) {
        self.alignment_offset = if self.multiline {
            0.0
        } else {
            let extra_space = (available_width - metrics.size.width).max(0.0);
            match self.alignment {
                TextAlignment::Start | TextAlignment::Justified => 0.0,
                TextAlignment::End => extra_space,
                TextAlignment::Center => extra_space / 2.0,
            }
        }
    }
}

impl<T: TextStorage + EditableText> Widget<T> for TextBox<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        self.suppress_adjust_hscroll = false;
        match event {
            Event::MouseDown(mouse) => {
                ctx.request_focus();
                ctx.set_active(true);
                let mut mouse = mouse.clone();
                mouse.pos += Vec2::new(self.hscroll_offset - self.alignment_offset, 0.0);

                if !mouse.focus {
                    self.was_focused_from_click = true;
                    self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                    self.editor.click(&mouse, data);
                }

                ctx.request_paint();
            }
            Event::MouseMove(mouse) => {
                let mut mouse = mouse.clone();
                mouse.pos += Vec2::new(self.hscroll_offset - self.alignment_offset, 0.0);
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    self.editor.drag(&mouse, data);
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
            Event::Command(ref cmd) if ctx.is_focused() && cmd.is(crate::commands::COPY) => {
                self.editor.copy(data);
                ctx.set_handled();
            }
            Event::Command(ref cmd) if ctx.is_focused() && cmd.is(crate::commands::CUT) => {
                self.editor.cut(data);
                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(TextBox::PERFORM_EDIT) => {
                let edit = cmd.get_unchecked(TextBox::PERFORM_EDIT);
                self.editor.do_edit(edit.to_owned(), data);
            }
            Event::Paste(ref item) => {
                if let Some(string) = item.get_string() {
                    self.editor.paste(string, data);
                }
            }
            Event::KeyDown(key_event) => {
                match key_event {
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KbKey::Tab).matches(k_e) => ctx.focus_next(),
                    k_e if HotKey::new(SysMods::Shift, KbKey::Tab).matches(k_e) => ctx.focus_prev(),
                    k_e => {
                        if let Some(edit) = self.input_handler.handle_event(k_e) {
                            self.suppress_adjust_hscroll = matches!(edit, EditAction::SelectAll);
                            self.editor.do_edit(edit, data);
                            // an explicit request update in case the selection
                            // state has changed, but the data hasn't.
                            ctx.request_update();
                            ctx.request_paint();
                        }
                    }
                };
                self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                ctx.register_for_focus();
                self.editor.set_text(data.to_owned());
                self.editor.rebuild_if_needed(ctx.text(), env);
            }
            LifeCycle::FocusChanged(is_focused) => {
                if MAC_OR_LINUX && *is_focused && !self.was_focused_from_click {
                    self.editor.select_all(data);
                }
                self.was_focused_from_click = false;
                self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        self.editor.update(ctx, data, env);
        if !self.suppress_adjust_hscroll && !self.multiline {
            self.update_hscroll(ctx.size().width, env);
        }
        if ctx.env_changed() && self.placeholder.needs_rebuild_after_update(ctx) {
            ctx.request_layout();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let width = env.get(theme::WIDE_WIDGET_WIDTH);
        let text_insets = env.get(theme::TEXTBOX_INSETS);

        self.placeholder.rebuild_if_needed(ctx.text(), env);
        if self.multiline {
            self.editor
                .set_wrap_width(bc.max().width - text_insets.x_value());
        }
        self.editor.rebuild_if_needed(ctx.text(), env);

        let text_metrics = if data.is_empty() {
            self.placeholder.layout_metrics()
        } else {
            self.editor.layout().layout_metrics()
        };

        let height = text_metrics.size.height + text_insets.y_value();
        let size = bc.constrain((width, height));
        // if we have a non-left text-alignment, we need to manually adjust our position.
        self.update_alignment_adjustment(size.width - text_insets.x_value(), &text_metrics);
        self.text_pos = Point::new(text_insets.x0 + self.alignment_offset, text_insets.y0);

        let bottom_padding = (size.height - text_metrics.size.height) / 2.0;
        let baseline_off =
            bottom_padding + (text_metrics.size.height - text_metrics.first_baseline);
        ctx.set_baseline_offset(baseline_off);

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let size = ctx.size();
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let selection_color = env.get(theme::SELECTION_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);
        let border_width = env.get(theme::TEXTBOX_BORDER_WIDTH);
        let text_insets = env.get(theme::TEXTBOX_INSETS);

        let is_focused = ctx.is_focused();

        let border_color = if is_focused {
            env.get(theme::PRIMARY_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        // Paint the background
        let clip_rect = Size::new(size.width - border_width, size.height)
            .to_rect()
            .inset(-border_width / 2.0)
            .to_rounded_rect(env.get(theme::TEXTBOX_BORDER_RADIUS));

        ctx.fill(clip_rect, &background_color);

        // Render text, selection, and cursor inside a clip
        ctx.with_save(|rc| {
            rc.clip(clip_rect);

            // Shift everything inside the clip by the hscroll_offset
            rc.transform(Affine::translate((-self.hscroll_offset, 0.)));

            let text_pos = self.text_position();
            // Draw selection rect
            if !data.is_empty() {
                if is_focused {
                    for sel in self.editor.selection_rects() {
                        let sel = sel + text_pos.to_vec2();
                        let rounded = sel.to_rounded_rect(1.0);
                        rc.fill(rounded, &selection_color);
                    }
                }
                self.editor.draw(rc, text_pos);
            } else {
                self.placeholder.draw(rc, text_pos);
            }

            // Paint the cursor if focused and there's no selection
            if is_focused && self.should_draw_cursor() {
                // if there's no data, we always draw the cursor based on
                // our alignment.
                let cursor = if data.is_empty() {
                    let dx = match self.alignment {
                        TextAlignment::Start | TextAlignment::Justified => text_insets.x0,
                        TextAlignment::Center => size.width / 2.0,
                        TextAlignment::End => size.width - text_insets.x1,
                    };
                    self.editor.cursor_line() + Vec2::new(dx, text_insets.y0)
                } else {
                    // the cursor position can extend past the edge of the layout
                    // (commonly when there is trailing whitespace) so we clamp it
                    // to the right edge.
                    let mut cursor = self.editor.cursor_line() + text_pos.to_vec2();
                    let dx = size.width + self.hscroll_offset - text_insets.x0 - cursor.p0.x;
                    if dx < 0.0 {
                        cursor = cursor + Vec2::new(dx, 0.);
                    }
                    cursor
                };
                rc.stroke(cursor, &cursor_color, 1.);
            }
        });

        // Paint the border
        ctx.stroke(clip_rect, &border_color, border_width);
    }
}

impl<T> Default for TextBox<T> {
    fn default() -> Self {
        TextBox::new()
    }
}
