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
use tracing::{instrument, trace};

use crate::piet::PietText;
use crate::text::{
    format::{Formatter, ValidationError},
    BasicTextInput, EditAction, EditableText, Editor, LayoutMetrics, Selection, TextInput,
    TextLayout, TextStorage,
};
use crate::widget::prelude::*;
use crate::{
    theme, Affine, Color, Cursor, Data, FontDescriptor, HotKey, KbKey, KeyOrValue, Point, Selector,
    SysMods, TextAlignment, TimerToken, Vec2,
};

const MAC_OR_LINUX: bool = cfg!(any(target_os = "macos", target_os = "linux"));
const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);

const BEGIN_EDITING: Selector = Selector::new("druid.builtin.textbox-begin-editing");
const COMPLETE_EDITING: Selector = Selector::new("druid.builtin.textbox-complete-editing");
const CANCEL_EDITING: Selector = Selector::new("druid.builtin.textbox-cancel-editing");

/// A widget that allows user text input.
///
/// # Editing values
///
/// If the text you are editing represents a value of some other type, such
/// as a number, you should use a [`ValueTextBox`] and an appropriate
/// [`Formatter`]. You can create a [`ValueTextBox`] by passing the appropriate
/// [`Formatter`] to [`TextBox::with_formatter`].
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

/// A `TextBox` that uses a [`Formatter`] to handle formatting and validation
/// of its data.
///
/// There are a number of ways to customize the behaviour of the text box
/// in relation to the provided [`Formatter`]:
///
/// - [`ValueTextBox::validate_while_editing`] takes a flag that determines whether
/// or not the textbox can display text that is not valid, while editing is
/// in progress. (Text will still be validated when the user attempts to complete
/// editing.)
///
/// - [`ValueTextBox::update_data_while_editing`] takes a flag that determines
/// whether the output value is updated during editing, when possible.
///
/// - [`ValueTextBox::delegate`] allows you to provide some implementation of
/// the [`ValidationDelegate`] trait, which receives a callback during editing;
/// this can be used to report errors further back up the tree.
pub struct ValueTextBox<T> {
    inner: TextBox<String>,
    formatter: Box<dyn Formatter<T>>,
    callback: Option<Box<dyn ValidationDelegate>>,
    is_editing: bool,
    validate_while_editing: bool,
    update_data_while_editing: bool,
    /// the last data that this textbox saw or created.
    /// This is used to determine when a change to the data is originating
    /// elsewhere in the application, which we need to special-case
    last_known_data: Option<T>,
    force_selection: Option<Selection>,
    old_buffer: String,
    buffer: String,
}

/// A type that can be registered to receive callbacks as the state of a
/// [`ValueTextBox`] changes.
pub trait ValidationDelegate {
    /// Called with a [`TextBoxEvent`] whenever the validation state of a
    /// [`ValueTextBox`] changes.
    fn event(&mut self, ctx: &mut EventCtx, event: TextBoxEvent, current_text: &str);
}

/// Events sent to a [`ValidationDelegate`].
pub enum TextBoxEvent {
    /// The textbox began editing.
    Began,
    /// An edit occured which was considered valid by the [`Formatter`].
    Changed,
    /// An edit occured which was rejected by the [`Formatter`].
    PartiallyInvalid(ValidationError),
    /// The user attempted to finish editing, but the input was not valid.
    Invalid(ValidationError),
    /// The user finished editing, with valid input.
    Complete,
    /// Editing was cancelled.
    Cancel,
}

impl TextBox<()> {
    /// Perform an `EditAction`.
    ///
    /// You can send a [`Command`] to a textbox containing an [`EditAction`]
    /// that the textbox should perform.
    ///
    /// [`Command`]: crate::Command
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

impl TextBox<String> {
    /// Turn this `TextBox` into a [`ValueTextBox`], using the [`Formatter`] to
    /// manage the value.
    ///
    /// For simple value formatting, you can use the [`ParseFormatter`].
    ///
    /// [`ValueTextBox`]: ValueTextBox
    /// [`Formatter`]: crate::text::format::Formatter
    /// [`ParseFormatter`]: crate::text::format::ParseFormatter
    pub fn with_formatter<T: Data>(
        self,
        formatter: impl Formatter<T> + 'static,
    ) -> ValueTextBox<T> {
        ValueTextBox::new(self, formatter)
    }
}

impl<T: TextStorage + EditableText> TextBox<T> {
    /// Set the textbox's selection.
    pub fn set_selection(&mut self, selection: Selection) {
        self.editor.set_selection(selection);
    }

    /// Set the text and force the editor to update.
    ///
    /// This should be rarely needed; the main use-case would be if you need
    /// to manually set the text and then immediately do hit-testing or other
    /// tasks that rely on having an up-to-date text layout.
    pub fn force_rebuild(&mut self, text: T, factory: &mut PietText, env: &Env) {
        self.editor.set_text(text);
        self.editor.rebuild_if_needed(factory, env);
    }

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
    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, event, data, _env))]
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

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, event, data, env))]
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

    #[instrument(
        name = "TextBox",
        level = "trace",
        skip(self, ctx, _old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.editor.update(ctx, data, env);
        if !self.suppress_adjust_hscroll && !self.multiline {
            self.update_hscroll(ctx.size().width, env);
        }
        if ctx.env_changed() && self.placeholder.needs_rebuild_after_update(ctx) {
            ctx.request_layout();
        }
    }

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, bc, data, env))]
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

        trace!(
            "Computed layout: size={}, baseline_offset={:?}",
            size,
            baseline_off
        );
        size
    }

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, data, env))]
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

impl<T: Data> ValueTextBox<T> {
    /// Create a new `ValueTextBox` from a normal [`TextBox`] and a [`Formatter`].
    ///
    /// [`TextBox`]: crate::widget::TextBox
    /// [`Formatter`]: crate::text::format::Formatter
    pub fn new(inner: TextBox<String>, formatter: impl Formatter<T> + 'static) -> Self {
        ValueTextBox {
            inner,
            formatter: Box::new(formatter),
            callback: None,
            is_editing: false,
            last_known_data: None,
            validate_while_editing: true,
            update_data_while_editing: false,
            old_buffer: String::new(),
            buffer: String::new(),
            force_selection: None,
        }
    }

    /// Builder-style method to set an optional [`ValidationDelegate`] on this
    /// textbox.
    pub fn delegate(mut self, delegate: impl ValidationDelegate + 'static) -> Self {
        self.callback = Some(Box::new(delegate));
        self
    }

    /// Builder-style method to set whether or not this text box validates
    /// its contents during editing.
    ///
    /// If `true` (the default) edits that fail validation
    /// ([`Formatter::validate_partial_input`]) will be rejected. If `false`,
    /// those edits will be accepted, and the text box will be updated.
    pub fn validate_while_editing(mut self, validate: bool) -> Self {
        self.validate_while_editing = validate;
        self
    }

    /// Builder-style method to set whether or not this text box updates the
    /// incoming data during editing.
    ///
    /// If `false` (the default) the data is only updated when editing completes.
    pub fn update_data_while_editing(mut self, flag: bool) -> Self {
        self.update_data_while_editing = flag;
        self
    }

    fn complete(&mut self, ctx: &mut EventCtx, data: &mut T) {
        match self.formatter.value(&self.buffer) {
            Ok(new_data) => {
                *data = new_data;
                self.buffer = self.formatter.format(data);
                self.is_editing = false;
                ctx.request_update();
                if ctx.has_focus() {
                    ctx.resign_focus();
                }
                self.send_event(ctx, TextBoxEvent::Complete);
            }
            Err(err) => {
                // don't tab away from here if we're editing
                if !ctx.has_focus() {
                    ctx.request_focus();
                }

                ctx.submit_command(
                    TextBox::PERFORM_EDIT
                        .with(EditAction::SelectAll)
                        .to(ctx.widget_id()),
                );
                self.send_event(ctx, TextBoxEvent::Invalid(err));
                // our content isn't valid
                // ideally we would flash the background or something
            }
        }
    }

    fn cancel(&mut self, ctx: &mut EventCtx, data: &T) {
        self.is_editing = false;
        self.buffer = self.formatter.format(data);
        ctx.request_update();
        ctx.resign_focus();
        self.send_event(ctx, TextBoxEvent::Cancel);
    }

    fn begin(&mut self, ctx: &mut EventCtx, data: &T) {
        self.is_editing = true;
        self.buffer = self.formatter.format_for_editing(data);
        self.last_known_data = Some(data.clone());
        ctx.request_update();
        self.send_event(ctx, TextBoxEvent::Began);
    }

    fn send_event(&mut self, ctx: &mut EventCtx, event: TextBoxEvent) {
        if let Some(delegate) = self.callback.as_mut() {
            delegate.event(ctx, event, &self.buffer)
        }
    }
}

impl<T: Data> Widget<T> for ValueTextBox<T> {
    #[instrument(
        name = "ValueTextBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if matches!(event, Event::Command(cmd) if cmd.is(BEGIN_EDITING)) {
            return self.begin(ctx, data);
        }

        if self.is_editing {
            // if we reject an edit we want to reset the selection
            let pre_sel = *self.inner.editor().selection();
            match event {
                Event::Command(cmd) if cmd.is(COMPLETE_EDITING) => return self.complete(ctx, data),
                Event::Command(cmd) if cmd.is(CANCEL_EDITING) => return self.cancel(ctx, data),
                Event::KeyDown(k_e) if HotKey::new(None, KbKey::Enter).matches(k_e) => {
                    ctx.set_handled();
                    self.complete(ctx, data);
                    return;
                }
                Event::KeyDown(k_e) if HotKey::new(None, KbKey::Escape).matches(k_e) => {
                    ctx.set_handled();
                    self.cancel(ctx, data);
                    return;
                }
                event => {
                    self.inner.event(ctx, event, &mut self.buffer, env);
                }
            }
            // if an edit occured, validate it with the formatter
            if self.buffer != self.old_buffer {
                let mut validation = self
                    .formatter
                    .validate_partial_input(&self.buffer, &self.inner.editor().selection());

                if self.validate_while_editing {
                    let new_buf = match (validation.text_change.take(), validation.is_err()) {
                        (Some(new_text), _) => {
                            // be helpful: if the formatter is misbehaved, log it.
                            if self
                                .formatter
                                .validate_partial_input(&new_text, &Selection::caret(0))
                                .is_err()
                            {
                                tracing::warn!(
                                    "formatter replacement text does not validate: '{}'",
                                    &new_text
                                );
                                None
                            } else {
                                Some(new_text)
                            }
                        }
                        (None, true) => Some(self.old_buffer.clone()),
                        _ => None,
                    };

                    let new_sel = match (validation.selection_change.take(), validation.is_err()) {
                        (Some(new_sel), _) => Some(new_sel),
                        (None, true) => Some(pre_sel),
                        _ => None,
                    };

                    if let Some(new_buf) = new_buf {
                        self.buffer = new_buf;
                    }
                    self.force_selection = new_sel;

                    if self.update_data_while_editing && !validation.is_err() {
                        if let Ok(new_data) = self.formatter.value(&self.buffer) {
                            *data = new_data;
                            self.last_known_data = Some(data.clone());
                        }
                    }
                }

                match validation.error() {
                    Some(err) => {
                        self.send_event(ctx, TextBoxEvent::PartiallyInvalid(err.to_owned()))
                    }
                    None => self.send_event(ctx, TextBoxEvent::Changed),
                };
                ctx.request_update();
            }
        } else if let Event::MouseDown(_) = event {
            self.begin(ctx, data);
            // we need to rebuild immediately here in order for the click
            // to be handled with the most recent text.
            self.inner
                .force_rebuild(self.buffer.clone(), ctx.text(), env);
            self.inner.event(ctx, event, &mut self.buffer, env);
        }
    }

    #[instrument(
        name = "ValueTextBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.buffer = self.formatter.format(data);
            self.old_buffer = self.buffer.clone();
        }
        self.inner.lifecycle(ctx, event, &self.buffer, env);

        if let LifeCycle::FocusChanged(focus) = event {
            // if the user focuses elsewhere, we need to reset ourselves
            if !focus {
                ctx.submit_command(COMPLETE_EDITING.to(ctx.widget_id()));
            } else if !self.is_editing {
                ctx.submit_command(BEGIN_EDITING.to(ctx.widget_id()));
            }
        }
    }

    #[instrument(
        name = "ValueTextBox",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        let changed_by_us = self
            .last_known_data
            .as_ref()
            .map(|d| d.same(data))
            .unwrap_or(false);
        if self.is_editing {
            if changed_by_us {
                self.inner.update(ctx, &self.old_buffer, &self.buffer, env);
                self.old_buffer = self.buffer.clone();
            } else {
                // textbox is not well equipped to deal with the fact that, in
                // druid, data can change anywhere in the tree. If we are actively
                // editing, and new data arrives, we ignore the new data and keep
                // editing; the alternative would be to cancel editing, which
                // could also make sense.
                tracing::warn!(
                    "ValueTextBox data changed externally, idk: '{}'",
                    self.formatter.format(data)
                );
            }
        } else {
            if !old_data.same(data) {
                // we aren't editing and data changed
                let new_text = self.formatter.format(data);
                self.old_buffer = std::mem::replace(&mut self.buffer, new_text);
            }

            if !self.old_buffer.same(&self.buffer) {
                // inner widget handles calling request_layout, as needed
                self.inner.update(ctx, &self.old_buffer, &self.buffer, env);
                self.old_buffer = self.buffer.clone();
            } else if ctx.env_changed() {
                self.inner.update(ctx, &self.buffer, &self.buffer, env);
            }
        }
        if let Some(sel) = self.force_selection.take() {
            self.inner.set_selection(sel);
        }
    }

    #[instrument(
        name = "ValueTextBox",
        level = "trace",
        skip(self, ctx, bc, _data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, &self.buffer, env)
    }

    #[instrument(name = "ValueTextBox", level = "trace", skip(self, ctx, _data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        self.inner.paint(ctx, &self.buffer, env);
    }
}

impl<T> Default for TextBox<T> {
    fn default() -> Self {
        TextBox::new()
    }
}
