// Copyright 2021 The Druid Authors.
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

//! A widget component that integrates with the platform text system.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ops::Range;
use std::sync::{Arc, Weak};

use tracing::instrument;

use super::{EditableText, ImeHandlerRef, Movement, Selection, TextLayout, TextStorage};
use crate::kurbo::{Line, Point, Rect, Vec2};
use crate::piet::TextLayout as _;
use crate::shell::text::{Action as ImeAction, Event as ImeUpdate, InputHandler};
use crate::widget::prelude::*;
use crate::{text, theme, Cursor, Env, Modifiers, Selector, TextAlignment, UpdateCtx};

/// A widget that accepts text input.
///
/// This is intended to be used as a component of other widgets.
///
/// Text input is more complicated than you think, probably. For a good
/// overview, see [`druid_shell::text`].
///
/// This type manages an inner [`EditSession`] that is shared with the platform.
/// Unlike other aspects of Druid, the platform interacts with this session, not
/// through discrete events.
///
/// This is managed through a simple 'locking' mechanism; the platform asks for
/// a lock on a particular text session that it wishes to interact with, calls
/// methods on the locked session, and then later releases the lock.
///
/// Importantly, *other events may be received while the lock is held*.
///
/// It is the responsibility of the user of this widget to ensure that the
/// session is not locked before it is accessed. This can be done by checking
/// [`TextComponent::can_read`] and [`TextComponent::can_write`];
/// after checking these methods the inner session can be accessed via
/// [`TextComponent::borrow`] and [`TextComponent::borrow_mut`].
///
/// Sementically, this functions like a `RefCell`; attempting to borrow while
/// a lock is held will result in a panic.
#[derive(Debug, Clone)]
pub struct TextComponent<T> {
    inner: Arc<RefCell<EditSession<T>>>,
    lock: Arc<Cell<ImeLock>>,
    // HACK: because of the way focus works (it is managed higher up, in
    // whatever widget is controlling this) we can't rely on `is_focused` in
    // the PaintCtx.
    /// A manual flag set by the parent to control drawing behaviour.
    ///
    /// The parent should update this when handling [`LifeCycle::FocusChanged`].
    pub has_focus: bool,
}

/// Editable text state.
///
/// This is the inner state of a [`TextComponent`]. It should only be accessed
/// through its containing [`TextComponent`], or by the platform through an
/// [`ImeHandlerRef`] created by [`TextComponent::input_handler`].
#[derive(Debug, Clone)]
pub struct EditSession<T> {
    /// The inner [`TextLayout`] object.
    ///
    /// This is exposed so that users can do things like set text properties;
    /// you should avoid doing things like rebuilding this layout manually, or
    /// setting the text directly.
    pub layout: TextLayout<T>,
    /// If the platform modifies the text, this contains the new text;
    /// we update the app `Data` with this text on the next update pass.
    external_text_change: Option<T>,
    external_selection_change: Option<Selection>,
    external_scroll_to: Option<bool>,
    external_action: Option<ImeAction>,
    /// A flag set in `update` if the text has changed from a non-IME source.
    pending_ime_invalidation: Option<ImeUpdate>,
    /// If `true`, the component will send the [`TextComponent::RETURN`]
    /// notification when the user enters a newline.
    pub send_notification_on_return: bool,
    /// If `true`, the component will send the [`TextComponent::CANCEL`]
    /// notification when the user cancels editing.
    pub send_notification_on_cancel: bool,
    selection: Selection,
    accepts_newlines: bool,
    accepts_tabs: bool,
    alignment: TextAlignment,
    /// The y-position of the text when it does not fill our width.
    alignment_offset: f64,
    /// The portion of the text that is currently marked by the IME.
    composition_range: Option<Range<usize>>,
    /// The origin of the textbox, relative to the origin of the window.
    pub origin: Point,
}

/// An object that can be used to acquire an `ImeHandler`.
///
/// This does not own the session; when the widget that owns the session
/// is dropped, this will become invalid.
#[derive(Debug, Clone)]
struct EditSessionRef<T> {
    inner: Weak<RefCell<EditSession<T>>>,
    lock: Arc<Cell<ImeLock>>,
}

/// A locked handle to an [`EditSession`].
///
/// This type implements [`InputHandler`]; it is the type that we pass to the
/// platform.
struct EditSessionHandle<T> {
    text: T,
    inner: Arc<RefCell<EditSession<T>>>,
}

/// An informal lock.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ImeLock {
    None,
    ReadWrite,
    Read,
}

impl<T: TextStorage + EditableText> ImeHandlerRef for EditSessionRef<T> {
    fn is_alive(&self) -> bool {
        Weak::strong_count(&self.inner) > 0
    }

    fn acquire(&self, mutable: bool) -> Option<Box<dyn InputHandler + 'static>> {
        let lock = if mutable {
            ImeLock::ReadWrite
        } else {
            ImeLock::Read
        };
        assert_eq!(
            self.lock.replace(lock),
            ImeLock::None,
            "Ime session is already locked"
        );
        Weak::upgrade(&self.inner)
            .map(EditSessionHandle::new)
            .map(|doc| Box::new(doc) as Box<dyn InputHandler>)
    }

    fn release(&self) -> bool {
        self.lock.replace(ImeLock::None) == ImeLock::ReadWrite
    }
}

impl TextComponent<()> {
    /// A notification sent by the component when the cursor has moved.
    ///
    /// If the payload is true, this follows an edit, and the view will need
    /// layout before scrolling.
    pub const SCROLL_TO: Selector<bool> = Selector::new("druid-builtin.textbox-scroll-to");

    /// A notification sent by the component when the user hits return.
    ///
    /// This is only sent when `send_notification_on_return` is `true`.
    pub const RETURN: Selector = Selector::new("druid-builtin.textbox-return");

    /// A notification sent when the user cancels editing.
    ///
    /// This is only sent when `send_notification_on_cancel` is `true`.
    pub const CANCEL: Selector = Selector::new("druid-builtin.textbox-cancel");

    /// A notification sent by the component when the user presses the tab key.
    ///
    /// This is not sent if `accepts_tabs` is true.
    ///
    /// An ancestor can handle this event in order to do things like request
    /// a focus change.
    pub const TAB: Selector = Selector::new("druid-builtin.textbox-tab");

    /// A notification sent by the component when the user inserts a backtab.
    ///
    /// This is not sent if `accepts_tabs` is true.
    ///
    /// An ancestor can handle this event in order to do things like request
    /// a focus change.
    pub const BACKTAB: Selector = Selector::new("druid-builtin.textbox-backtab");
}

impl<T> TextComponent<T> {
    /// Returns `true` if the inner [`EditSession`] can be read.
    pub fn can_read(&self) -> bool {
        self.lock.get() != ImeLock::ReadWrite
    }

    /// Returns `true` if the inner [`EditSession`] can be mutated.
    pub fn can_write(&self) -> bool {
        self.lock.get() == ImeLock::None
    }

    /// Attempt to mutably borrow the inner [`EditSession`].
    ///
    /// # Panics
    ///
    /// This method panics if there is an outstanding lock on the session.
    pub fn borrow_mut(&self) -> RefMut<'_, EditSession<T>> {
        assert!(self.can_write());
        self.inner.borrow_mut()
    }

    /// Attempt to borrow the inner [`EditSession`].
    ///
    /// # Panics
    ///
    /// This method panics if there is an outstanding write lock on the session.
    pub fn borrow(&self) -> Ref<'_, EditSession<T>> {
        assert!(self.can_read());
        self.inner.borrow()
    }
}

impl<T: EditableText + TextStorage> TextComponent<T> {
    /// Returns an [`ImeHandlerRef`] that can accept platform text input.
    ///
    /// The widget managing this component should call [`LifeCycleCtx::register_text_input`]
    /// during [`LifeCycle::WidgetAdded`], and pass it this object.
    pub fn input_handler(&self) -> impl ImeHandlerRef {
        EditSessionRef {
            inner: Arc::downgrade(&self.inner),
            lock: self.lock.clone(),
        }
    }
}

impl<T: TextStorage + EditableText> Widget<T> for TextComponent<T> {
    #[instrument(
        name = "InputComponent",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(mouse) if self.can_write() => {
                ctx.set_active(true);
                // ensure data is up to date before a click
                let needs_rebuild = self
                    .borrow()
                    .layout
                    .text()
                    .map(|old| !old.same(data))
                    .unwrap_or(true);
                if needs_rebuild {
                    self.borrow_mut().layout.set_text(data.clone());
                    self.borrow_mut().layout.rebuild_if_needed(ctx.text(), env);
                    self.borrow_mut()
                        .update_pending_invalidation(ImeUpdate::Reset);
                }
                self.borrow_mut()
                    .do_mouse_down(mouse.pos, mouse.mods, mouse.count);
                self.borrow_mut()
                    .update_pending_invalidation(ImeUpdate::SelectionChanged);
                ctx.request_update();
            }
            Event::MouseMove(mouse) if self.can_write() => {
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    let pre_sel = self.borrow().selection();
                    self.borrow_mut().do_drag(mouse.pos);
                    if self.borrow().selection() != pre_sel {
                        self.borrow_mut()
                            .update_pending_invalidation(ImeUpdate::SelectionChanged);
                        ctx.request_update();
                        ctx.request_paint();
                    }
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            Event::ImeStateChange => {
                assert!(
                    self.can_write(),
                    "lock release should be cause of ImeStateChange event"
                );
                let scroll_to = self.borrow_mut().take_scroll_to();
                let action = self.borrow_mut().take_external_action();

                if let Some(scroll_to) = scroll_to {
                    ctx.submit_notification(TextComponent::SCROLL_TO.with(scroll_to));
                }
                if let Some(action) = action {
                    match action {
                        ImeAction::Cancel => ctx.submit_notification(TextComponent::CANCEL),
                        ImeAction::InsertNewLine { .. } => {
                            ctx.submit_notification(TextComponent::RETURN)
                        }
                        ImeAction::InsertTab { .. } => ctx.submit_notification(TextComponent::TAB),
                        ImeAction::InsertBacktab => ctx.submit_notification(TextComponent::BACKTAB),
                        _ => tracing::warn!("unexepcted external action '{:?}'", action),
                    };
                }
                let text = self.borrow_mut().take_external_text_change();
                let selection = self.borrow_mut().take_external_selection_change();
                if let Some(text) = text {
                    self.borrow_mut().layout.set_text(text.clone());
                    *data = text;
                }
                if let Some(selection) = selection {
                    self.borrow_mut().selection = selection;
                }
                ctx.request_update();
            }
            _ => (),
        }
    }

    #[instrument(
        name = "InputComponent",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                assert!(
                    self.can_write(),
                    "ime should never be locked at WidgetAdded"
                );
                self.borrow_mut().layout.set_text(data.to_owned());
                self.borrow_mut().layout.rebuild_if_needed(ctx.text(), env);
            }
            //FIXME: this should happen in the parent too?
            LifeCycle::Internal(crate::InternalLifeCycle::ParentWindowOrigin)
                if self.can_write() =>
            {
                if self.can_write() {
                    let prev_origin = self.borrow().origin;
                    let new_origin = ctx.window_origin();
                    if prev_origin != new_origin {
                        self.borrow_mut().origin = ctx.window_origin();
                        ctx.invalidate_text_input(ImeUpdate::LayoutChanged);
                    }
                }
            }
            _ => (),
        }
    }

    #[instrument(
        name = "InputComponent",
        level = "trace",
        skip(self, ctx, _old, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old: &T, data: &T, env: &Env) {
        if self.can_write() {
            self.borrow_mut().update(ctx, data, env);
        }
    }

    #[instrument(
        name = "InputComponent",
        level = "trace",
        skip(self, ctx, bc, _data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        if !self.can_write() {
            tracing::warn!("Text layout called with IME lock held.");
            return Size::ZERO;
        }

        self.borrow_mut().layout.set_wrap_width(bc.max().width);
        self.borrow_mut().layout.rebuild_if_needed(ctx.text(), env);
        let metrics = self.borrow().layout.layout_metrics();
        let width = if bc.max().width.is_infinite() || bc.max().width < f64::MAX {
            metrics.trailing_whitespace_width
        } else {
            metrics.size.width
        };
        let size = bc.constrain((width, metrics.size.height));
        let extra_width = if self.borrow().accepts_newlines {
            0.0
        } else {
            (size.width - width).max(0.0)
        };
        self.borrow_mut().update_alignment_offset(extra_width);
        let baseline_off = metrics.size.height - metrics.first_baseline;
        ctx.set_baseline_offset(baseline_off);
        size
    }

    #[instrument(name = "InputComponent", level = "trace", skip(self, ctx, _data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        if !self.can_read() {
            tracing::warn!("Text paint called with IME lock held.");
        }

        let selection_color = if self.has_focus {
            env.get(theme::SELECTED_TEXT_BACKGROUND_COLOR)
        } else {
            env.get(theme::SELECTED_TEXT_INACTIVE_BACKGROUND_COLOR)
        };

        let cursor_color = env.get(theme::CURSOR_COLOR);
        let text_offset = Vec2::new(self.borrow().alignment_offset, 0.0);

        let selection = self.borrow().selection();
        let composition = self.borrow().composition_range();
        let sel_rects = self.borrow().layout.rects_for_range(selection.range());
        if let Some(composition) = composition {
            // I believe selection should always be contained in composition range while composing?
            assert!(composition.start <= selection.anchor && composition.end >= selection.active);
            let comp_rects = self.borrow().layout.rects_for_range(composition);
            for region in comp_rects {
                let y = region.max_y().floor();
                let line = Line::new((region.min_x(), y), (region.max_x(), y)) + text_offset;
                ctx.stroke(line, &cursor_color, 1.0);
            }
            for region in sel_rects {
                let y = region.max_y().floor();
                let line = Line::new((region.min_x(), y), (region.max_x(), y)) + text_offset;
                ctx.stroke(line, &cursor_color, 2.0);
            }
        } else {
            for region in sel_rects {
                let rounded = (region + text_offset).to_rounded_rect(1.0);
                ctx.fill(rounded, &selection_color);
            }
        }
        self.borrow().layout.draw(ctx, text_offset.to_point());
    }
}

impl<T> EditSession<T> {
    /// The current [`Selection`].
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Manually set the selection.
    ///
    /// If the new selection is different from the current selection, this
    /// will return an ime event that the controlling widget should use to
    /// invalidte the platform's IME state, by passing it to
    /// [`EventCtx::invalidate_text_input`].
    #[must_use]
    pub fn set_selection(&mut self, selection: Selection) -> Option<ImeUpdate> {
        if selection != self.selection {
            self.selection = selection;
            self.update_pending_invalidation(ImeUpdate::SelectionChanged);
            Some(ImeUpdate::SelectionChanged)
        } else {
            None
        }
    }

    /// The range of text currently being modified by an IME.
    pub fn composition_range(&self) -> Option<Range<usize>> {
        self.composition_range.clone()
    }

    /// Sets whether or not this session will allow the insertion of newlines.
    pub fn set_accepts_newlines(&mut self, accepts_newlines: bool) {
        self.accepts_newlines = accepts_newlines;
    }

    /// Set the text alignment.
    ///
    /// This is only meaningful for single-line text that does not fill
    /// the minimum layout size.
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.alignment = alignment;
    }

    /// Returns any invalidation action that should be passed to the platform.
    ///
    /// The user of this component *must* check this after calling `update`.
    pub fn pending_ime_invalidation(&mut self) -> Option<ImeUpdate> {
        self.pending_ime_invalidation.take()
    }

    fn take_external_text_change(&mut self) -> Option<T> {
        self.external_text_change.take()
    }

    fn take_external_selection_change(&mut self) -> Option<Selection> {
        self.external_selection_change.take()
    }

    fn take_scroll_to(&mut self) -> Option<bool> {
        self.external_scroll_to.take()
    }

    fn take_external_action(&mut self) -> Option<ImeAction> {
        self.external_action.take()
    }

    // we don't want to replace a more aggressive invalidation with a less aggressive one.
    fn update_pending_invalidation(&mut self, new_invalidation: ImeUpdate) {
        self.pending_ime_invalidation = match self.pending_ime_invalidation.take() {
            None => Some(new_invalidation),
            Some(prev) => match (prev, new_invalidation) {
                (ImeUpdate::SelectionChanged, ImeUpdate::SelectionChanged) => {
                    ImeUpdate::SelectionChanged
                }
                (ImeUpdate::LayoutChanged, ImeUpdate::LayoutChanged) => ImeUpdate::LayoutChanged,
                _ => ImeUpdate::Reset,
            }
            .into(),
        }
    }

    fn update_alignment_offset(&mut self, extra_width: f64) {
        self.alignment_offset = match self.alignment {
            TextAlignment::Start | TextAlignment::Justified => 0.0,
            TextAlignment::End => extra_width,
            TextAlignment::Center => extra_width / 2.0,
        };
    }
}

impl<T: TextStorage + EditableText> EditSession<T> {
    /// Insert text *not* from the IME, replacing the current selection.
    ///
    /// The caller is responsible for notifying the platform of the change in
    /// text state, by calling [`EventCtx::invalidate_text_input`].
    #[must_use]
    pub fn insert_text(&mut self, data: &mut T, new_text: &str) -> ImeUpdate {
        let new_cursor_pos = self.selection.min() + new_text.len();
        data.edit(self.selection.range(), new_text);
        self.selection = Selection::caret(new_cursor_pos);
        self.scroll_to_selection_end(true);
        ImeUpdate::Reset
    }

    /// Sets the clipboard to the contents of the current selection.
    ///
    /// Returns `true` if the clipboard was set, and `false` if not (indicating)
    /// that the selection was empty.)
    pub fn set_clipboard(&self) -> bool {
        if let Some(text) = self
            .layout
            .text()
            .and_then(|txt| txt.slice(self.selection.range()))
        {
            if !text.is_empty() {
                crate::Application::global().clipboard().put_string(text);
                return true;
            }
        }
        false
    }

    fn scroll_to_selection_end(&mut self, after_edit: bool) {
        self.external_scroll_to = Some(after_edit);
    }

    fn do_action(&mut self, buffer: &mut T, action: ImeAction) {
        match action {
            ImeAction::Move(movement) => {
                let sel = text::movement(movement, self.selection, &self.layout, false);
                self.external_selection_change = Some(sel);
                self.scroll_to_selection_end(false);
            }
            ImeAction::MoveSelecting(movement) => {
                let sel = text::movement(movement, self.selection, &self.layout, true);
                self.external_selection_change = Some(sel);
                self.scroll_to_selection_end(false);
            }
            ImeAction::SelectAll => {
                let len = buffer.len();
                self.external_selection_change = Some(Selection::new(0, len));
            }
            ImeAction::SelectWord => {
                if self.selection.is_caret() {
                    let range =
                        text::movement::word_range_for_pos(buffer.as_str(), self.selection.active);
                    self.external_selection_change = Some(Selection::new(range.start, range.end));
                }

                // it is unclear what the behaviour should be if the selection
                // is not a caret (and may span multiple words)
            }
            // This requires us to have access to the layout, which might be stale?
            ImeAction::SelectLine => (),
            // this assumes our internal selection is consistent with the buffer?
            ImeAction::SelectParagraph => {
                if !self.selection.is_caret() || buffer.len() < self.selection.active {
                    return;
                }
                let prev = buffer.preceding_line_break(self.selection.active);
                let next = buffer.next_line_break(self.selection.active);
                self.external_selection_change = Some(Selection::new(prev, next));
            }
            ImeAction::Delete(movement) if self.selection.is_caret() => {
                if movement == Movement::Grapheme(druid_shell::text::Direction::Upstream) {
                    self.backspace(buffer);
                } else {
                    let to_delete = text::movement(movement, self.selection, &self.layout, true);
                    self.selection = to_delete;
                    self.ime_insert_text(buffer, "")
                }
            }
            ImeAction::Delete(_) => self.ime_insert_text(buffer, ""),
            ImeAction::DecomposingBackspace => {
                tracing::warn!("Decomposing Backspace is not implemented");
                self.backspace(buffer);
            }
            //ImeAction::UppercaseSelection
            //| ImeAction::LowercaseSelection
            //| ImeAction::TitlecaseSelection => {
            //tracing::warn!("IME transformations are not implemented");
            //}
            ImeAction::InsertNewLine {
                newline_type,
                ignore_hotkey,
            } => {
                if self.send_notification_on_return && !ignore_hotkey {
                    self.external_action = Some(action);
                } else if self.accepts_newlines {
                    self.ime_insert_text(buffer, &newline_type.to_string());
                }
            }
            ImeAction::InsertTab { ignore_hotkey } => {
                if ignore_hotkey || self.accepts_tabs {
                    self.ime_insert_text(buffer, "\t");
                } else if !ignore_hotkey {
                    self.external_action = Some(action);
                }
            }
            ImeAction::InsertBacktab => {
                if !self.accepts_tabs {
                    self.external_action = Some(action);
                }
            }
            ImeAction::InsertSingleQuoteIgnoringSmartQuotes => self.ime_insert_text(buffer, "'"),
            ImeAction::InsertDoubleQuoteIgnoringSmartQuotes => self.ime_insert_text(buffer, "\""),
            ImeAction::Cancel if self.send_notification_on_cancel => {
                self.external_action = Some(action)
            }
            other => tracing::warn!("unhandled IME action {:?}", other),
        }
    }

    /// Replace the current selection with `text`, and advance the cursor.
    ///
    /// This should only be called from the IME.
    fn ime_insert_text(&mut self, buffer: &mut T, text: &str) {
        let new_cursor_pos = self.selection.min() + text.len();
        buffer.edit(self.selection.range(), text);
        self.external_selection_change = Some(Selection::caret(new_cursor_pos));
        self.scroll_to_selection_end(true);
    }

    fn backspace(&mut self, buffer: &mut T) {
        let to_del = if self.selection.is_caret() {
            let del_start = text::offset_for_delete_backwards(&self.selection, buffer);
            del_start..self.selection.anchor
        } else {
            self.selection.range()
        };
        self.external_selection_change = Some(Selection::caret(to_del.start));
        buffer.edit(to_del, "");
        self.scroll_to_selection_end(true);
    }

    fn do_mouse_down(&mut self, point: Point, mods: Modifiers, count: u8) {
        let point = point + Vec2::new(self.alignment_offset, 0.0);
        let pos = self.layout.text_position_for_point(point);
        if mods.shift() {
            self.selection.active = pos;
        } else {
            let sel = self.sel_region_for_pos(pos, count);
            self.selection.anchor = sel.start;
            self.selection.active = sel.end;
        }
    }

    fn do_drag(&mut self, point: Point) {
        let point = point + Vec2::new(self.alignment_offset, 0.0);
        //FIXME: this should behave differently if we were double or triple clicked
        let pos = self.layout.text_position_for_point(point);
        self.selection.active = pos;
        self.scroll_to_selection_end(false);
    }

    /// Returns a line suitable for drawing a standard cursor.
    pub fn cursor_line_for_text_position(&self, pos: usize) -> Line {
        let line = self.layout.cursor_line_for_text_position(pos);
        line + Vec2::new(self.alignment_offset, 0.0)
    }

    fn sel_region_for_pos(&mut self, pos: usize, click_count: u8) -> Range<usize> {
        match click_count {
            1 => pos..pos,
            2 => self.word_for_pos(pos),
            _ => {
                let text = match self.layout.text() {
                    Some(text) => text,
                    None => return pos..pos,
                };
                let line_min = text.preceding_line_break(pos);
                let line_max = text.next_line_break(pos);
                line_min..line_max
            }
        }
    }

    fn word_for_pos(&self, pos: usize) -> Range<usize> {
        let layout = match self.layout.layout() {
            Some(layout) => layout,
            None => return pos..pos,
        };

        let line_n = layout.hit_test_text_position(pos).line;
        let lm = layout.line_metric(line_n).unwrap();
        let text = layout.line_text(line_n).unwrap();
        let rel_pos = pos - lm.start_offset;
        let mut range = text::movement::word_range_for_pos(text, rel_pos);
        range.start += lm.start_offset;
        range.end += lm.start_offset;
        range
    }

    fn update(&mut self, ctx: &mut UpdateCtx, new_data: &T, env: &Env) {
        if self
            .layout
            .text()
            .as_ref()
            .map(|t| !t.same(new_data))
            .unwrap_or(true)
        {
            self.update_pending_invalidation(ImeUpdate::Reset);
            self.layout.set_text(new_data.clone());
        }
        if self.layout.needs_rebuild_after_update(ctx) {
            ctx.request_layout();
        }
        let new_sel = self.selection.constrained(new_data.as_str());
        if new_sel != self.selection {
            self.selection = new_sel;
            self.update_pending_invalidation(ImeUpdate::SelectionChanged);
        }
        self.layout.rebuild_if_needed(ctx.text(), env);
    }
}

impl<T: TextStorage> EditSessionHandle<T> {
    fn new(inner: Arc<RefCell<EditSession<T>>>) -> Self {
        let text = inner.borrow().layout.text().cloned().unwrap();
        EditSessionHandle { inner, text }
    }
}

impl<T: TextStorage + EditableText> InputHandler for EditSessionHandle<T> {
    fn selection(&self) -> Selection {
        self.inner.borrow().selection
    }

    fn set_selection(&mut self, selection: Selection) {
        self.inner.borrow_mut().external_selection_change = Some(selection);
        self.inner.borrow_mut().external_scroll_to = Some(true);
    }

    fn composition_range(&self) -> Option<Range<usize>> {
        self.inner.borrow().composition_range.clone()
    }

    fn set_composition_range(&mut self, range: Option<Range<usize>>) {
        self.inner.borrow_mut().composition_range = range;
    }

    fn is_char_boundary(&self, i: usize) -> bool {
        self.text.cursor(i).is_some()
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn slice(&self, range: Range<usize>) -> std::borrow::Cow<str> {
        self.text.slice(range).unwrap()
    }

    fn replace_range(&mut self, range: Range<usize>, text: &str) {
        self.text.edit(range, text);
        self.inner.borrow_mut().external_text_change = Some(self.text.clone());
    }

    fn hit_test_point(&self, point: Point) -> crate::piet::HitTestPoint {
        self.inner
            .borrow()
            .layout
            .layout()
            .map(|layout| layout.hit_test_point(point))
            .unwrap_or_default()
    }

    fn line_range(&self, index: usize, _affinity: druid_shell::text::Affinity) -> Range<usize> {
        let inner = self.inner.borrow();
        let layout = inner.layout.layout().unwrap();
        let hit = layout.hit_test_text_position(index);
        let metric = layout.line_metric(hit.line).unwrap();
        metric.range()
    }

    fn bounding_box(&self) -> Option<Rect> {
        let size = self.inner.borrow().layout.size();
        Some(Rect::from_origin_size(self.inner.borrow().origin, size))
    }

    fn slice_bounding_box(&self, range: Range<usize>) -> Option<Rect> {
        let origin = self.inner.borrow().origin;
        let layout = &self.inner.borrow().layout;
        layout
            .rects_for_range(range)
            .first()
            .map(|rect| *rect + origin.to_vec2())
    }

    fn handle_action(&mut self, action: ImeAction) {
        self.inner.borrow_mut().do_action(&mut self.text, action);
        let text_changed = self
            .inner
            .borrow()
            .layout
            .text()
            .map(|old| !old.same(&self.text))
            .unwrap_or(true);
        if text_changed {
            self.inner.borrow_mut().external_text_change = Some(self.text.clone());
        }
    }
}

impl<T> Default for TextComponent<T> {
    fn default() -> Self {
        let inner = EditSession {
            layout: TextLayout::new(),
            external_scroll_to: None,
            external_text_change: None,
            external_selection_change: None,
            external_action: None,
            pending_ime_invalidation: None,
            selection: Selection::caret(0),
            composition_range: None,
            send_notification_on_return: false,
            send_notification_on_cancel: false,
            accepts_newlines: false,
            accepts_tabs: false,
            alignment: TextAlignment::Start,
            alignment_offset: 0.0,
            origin: Point::ZERO,
        };

        TextComponent {
            inner: Arc::new(RefCell::new(inner)),
            lock: Arc::new(Cell::new(ImeLock::None)),
            has_focus: false,
        }
    }
}
