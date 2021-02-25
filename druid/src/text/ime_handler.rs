// Copyright 2020 The Druid Authors.
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

//! A widget for text editing.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ops::Range;
use std::sync::{Arc, Weak};
use std::time::Duration;

use super::{EditableText, Movement, Selection, TextLayout, TextStorage};
use crate::kurbo::{Line, Point, Rect, Vec2};
use crate::piet::TextLayout as _;
use crate::shell::text::{Action as ImeAction, Event as ImeUpdate, InputHandler};
use crate::widget::prelude::*;
use crate::{
    theme, Cursor, Env, HotKey, KbKey, Modifiers, Selector, SysMods, TextAlignment, TimerToken,
    UpdateCtx,
};

const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);
const MAC_OR_LINUX: bool = cfg!(any(target_os = "macos", target_os = "linux"));

/// A widget that accepts text input.
///
/// This is intended to be used as a component of other widgets.
///
/// Text input is more complicated than you think, probably. For a good
/// overview, see [`druid_shell::text`].
///
/// This type manages an inner [`EditSession`] that is shared with the platform.
/// Unlike other aspects of druid, the platform interacts with this session, not
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
/// [`SharedTextComponent::can_read`] and [`SharedTextComponent::can_write`];
/// after checking these methods the inner session can be accessed via
/// [`SharedTextComponent::borrow`] and [`SharedTextComponent::borrow_mut`].
///
/// Sementically, this functions like a `RefCell`; attempting to borrow while
/// a lock is held will result in a panic.
#[derive(Debug, Clone)]
pub struct SharedTextComponent<T> {
    inner: Arc<RefCell<EditSession<T>>>,
    lock: Arc<Cell<ImeLock>>,
    /// true if a click event caused us to gain focus.
    ///
    /// On macOS, if focus happens via click then we set the selection based
    /// on the click position; if focus happens automatically (e.g. on tab)
    /// then we select our entire contents.
    was_focused_from_click: bool,
    cursor_on: bool,
    cursor_timer: TimerToken,
}

/// The inner state of an `EditSession`.
///
/// This may be modified directly, or it may be modified by the platform.
#[derive(Debug, Clone)]
pub struct EditSession<T> {
    pub layout: TextLayout<T>,
    /// If the platform modifies the text, this contains the new text;
    /// we update the app `Data` with this text on the next update pass.
    external_text_change: Option<T>,
    external_scroll_to: Option<bool>,
    selection: Selection,
    accepts_newlines: bool,
    alignment: TextAlignment,
    /// The y-position of the text when it does not fill our width.
    alignment_offset: f64,
    /// The portion of the text that is currently marked by the IME.
    composition_range: Option<Range<usize>>,
    /// The origin of the textbox, relative to the origin of the window.
    pub origin: Point,
}

/// A trait for input handlers registered by widgets.
///
/// A widget registers itself as accepting text input by calling
/// [`LifeCycleCtx::register_text_field`](crate::LifeCycleCtx::register_text_field)
/// while handling the [`LifeCycle::WidgetAdded`] event.
///
/// The widget does not explicitly *deregister* afterwards; rather anytime
/// the widget tree changes, druid will call [`is_alive`] on each registered
/// `ImeHandlerRef`, and deregister those that return `false`.
pub trait ImeHandlerRef {
    /// Returns `true` if this handler is still active.
    fn is_alive(&self) -> bool;
    /// Mark the session as locked, and return a handle.
    ///
    /// The lock can be read-write or read-only, indicated by the `mutable` flag.
    ///
    /// if [`is_alive`] is `true`, this should always return `Some(_)`.
    fn acquire(&self, mutable: bool) -> Option<Box<dyn InputHandler + 'static>>;
    /// Mark the session as released.
    fn release(&self) -> bool;
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

impl<T: EditableText + TextStorage> SharedTextComponent<T> {
    fn weak_ref(&self) -> impl ImeHandlerRef {
        EditSessionRef {
            inner: Arc::downgrade(&self.inner),
            lock: self.lock.clone(),
        }
    }
}

impl SharedTextComponent<()> {
    /// A notification sent by the component when its parent should redraw the cursor.
    pub const REDRAW_CURSOR: Selector = Selector::new("druid-builtin.textbox-redraw-cursor");
    /// If the payload is true, this follows an edit, and the view will need to be laid
    /// out before scrolling.
    pub const SCROLL_TO: Selector<bool> = Selector::new("druid-builtin.textbox-scroll-to");
}

impl<T> SharedTextComponent<T> {
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

    /// Returns `true` if the cursor should be drawn.
    pub fn should_draw_cursor(&self) -> bool {
        if cfg!(target_os = "macos") && self.can_read() {
            self.cursor_on && self.borrow().selection().is_caret()
        } else {
            self.cursor_on
        }
    }

    fn reset_cursor_blink(&mut self, token: TimerToken) {
        self.cursor_on = true;
        self.cursor_timer = token;
    }
}

impl<T: TextStorage + EditableText> Widget<T> for SharedTextComponent<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(mouse) if self.can_write() => {
                ctx.request_focus();
                ctx.set_active(true);
                if !mouse.focus {
                    self.was_focused_from_click = true;
                    self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                    self.borrow_mut().do_mouse_down(mouse.pos, mouse.mods, mouse.count);
                    ctx.invalidate_text_input(Some(ImeUpdate::SelectionChanged));
                }
                ctx.request_paint();
                ctx.submit_notification(SharedTextComponent::REDRAW_CURSOR);
            }
            Event::MouseMove(mouse) if self.can_write() => {
                ctx.set_cursor(&Cursor::IBeam);
                if ctx.is_active() {
                    let pre_sel = self.borrow().selection();
                    self.borrow_mut().do_drag(mouse.pos);
                    if self.borrow().selection() != pre_sel {
                        ctx.invalidate_text_input(Some(ImeUpdate::SelectionChanged));
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
            Event::Timer(id) => {
                if *id == self.cursor_timer {
                    self.cursor_on = !self.cursor_on;
                    ctx.request_paint();
                    self.cursor_timer = ctx.request_timer(CURSOR_BLINK_DURATION);
                    ctx.submit_notification(SharedTextComponent::REDRAW_CURSOR);
                }
            }
            Event::ImeStateChange => {
                assert!(self.can_write(), "lock release should be cause of ImeStateChange event");
                let text = self.borrow_mut().take_external_text_change();
                let scroll_to = self.borrow_mut().take_scroll_to();
                if let Some(text) = text {
                    *data = text;
                    ctx.request_layout();
                }

                if let Some(scroll_to) = scroll_to {
                    ctx.submit_notification(SharedTextComponent::SCROLL_TO.with(scroll_to));
                }
                self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
            }
            Event::KeyDown(key_event)
                    // if composing, let IME handle tab
                if self.can_read()
                    && self.borrow().composition_range().is_none() =>
            {
                match key_event {
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KbKey::Tab).matches(k_e) => {
                        ctx.focus_next();
                        ctx.set_handled();
                    }
                    k_e if HotKey::new(SysMods::Shift, KbKey::Tab).matches(k_e) => {
                        ctx.focus_prev();
                        ctx.set_handled();
                    }
                    _ => (),
                };
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                assert!(
                    self.can_write(),
                    "ime should never be locked at WidgetAdded"
                );
                ctx.register_text_input(self.weak_ref());
                self.borrow_mut().layout.set_text(data.to_owned());
                self.borrow_mut().layout.rebuild_if_needed(ctx.text(), env);
            }
            LifeCycle::Internal(crate::InternalLifeCycle::ParentWindowOrigin)
                if self.can_write() =>
            {
                if self.can_write() {
                    let prev_origin = self.borrow().origin;
                    let new_origin = ctx.window_origin();
                    if prev_origin != new_origin {
                        self.borrow_mut().origin = ctx.window_origin();
                        ctx.invalidate_text_input(Some(ImeUpdate::LayoutChanged));
                    }
                }
            }
            LifeCycle::FocusChanged(is_focused) => {
                if self.can_write() {
                    if MAC_OR_LINUX && *is_focused && !self.was_focused_from_click {
                        self.borrow_mut().selection = Selection::new(0, data.len());
                    }
                } else {
                    log::warn!("IME locked during FocusChanged");
                }
                self.was_focused_from_click = false;
                self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        if self.can_write() {
            self.borrow_mut().update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        if !self.can_write() {
            log::warn!("Text layout called with IME lock held.");
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

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        if !self.can_read() {
            log::warn!("Text paint called with IME lock held.");
        }
        let selection_color = env.get(theme::SELECTION_COLOR);
        let cursor_color = env.get(theme::CURSOR_COLOR);
        let text_offset = Vec2::new(self.borrow().alignment_offset, 0.0);

        let selection = self.borrow().selection();
        let composition = self.borrow().composition_range();
        let sel_rects = self.borrow().layout.rects_for_range(selection.range());
        if let Some(composition) = composition {
            // I believe selection should always be contained in composition range while composing?
            assert!(composition.start <= selection.start && composition.end >= selection.end);
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

    fn take_external_text_change(&mut self) -> Option<T> {
        self.external_text_change.take()
    }

    fn take_scroll_to(&mut self) -> Option<bool> {
        self.external_scroll_to.take()
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
    fn scroll_to_selection_end(&mut self, after_edit: bool) {
        self.external_scroll_to = Some(after_edit);
    }

    fn do_action(&mut self, buffer: &mut T, action: ImeAction) {
        //tracing::debug!("action {:?}", &action);
        match action {
            ImeAction::Move(movement) => {
                let sel =
                    crate::text::movement(movement.into(), self.selection, &self.layout, false);
                self.selection = sel;
                self.scroll_to_selection_end(false);
            }
            ImeAction::MoveSelecting(movement) => {
                let sel =
                    crate::text::movement(movement.into(), self.selection, &self.layout, true);
                self.selection = sel;
                self.scroll_to_selection_end(false);
            }
            ImeAction::SelectAll => {
                let len = self.layout.text().as_ref().map(|t| t.len()).unwrap_or(0);
                self.selection = Selection::new(0, len);
            }
            //ImeAction::SelectLine | ImeAction::SelectParagraph | ImeAction::SelectWord => {
            //tracing::warn!("Line/Word selection actions are not implemented");
            //}
            ImeAction::Delete(movement) if self.selection.is_caret() => {
                let movement: Movement = movement.into();
                if movement == Movement::Left {
                    self.backspace(buffer);
                } else {
                    let to_delete =
                        crate::text::movement(movement, self.selection, &self.layout, true);
                    self.selection = to_delete;
                    self.insert_text(buffer, "")
                }
            }
            ImeAction::Delete(_) => self.insert_text(buffer, ""),
            ImeAction::DecomposingBackspace => {
                log::warn!("Decomposing Backspace is not implemented");
                self.backspace(buffer);
            }
            //ImeAction::UppercaseSelection
            //| ImeAction::LowercaseSelection
            //| ImeAction::TitlecaseSelection => {
            //log::warn!("IME transformations are not implemented");
            //}
            ImeAction::InsertNewLine { newline_type, .. } if self.accepts_newlines => {
                self.insert_text(buffer, &newline_type.to_string());
            }
            ImeAction::InsertTab { .. } => {
                self.insert_text(buffer, "\t");
            }
            //ImeAction::InsertBacktab => log::warn!("IME backtab not implemented"),
            ImeAction::InsertSingleQuoteIgnoringSmartQuotes => self.insert_text(buffer, "'"),
            ImeAction::InsertDoubleQuoteIgnoringSmartQuotes => self.insert_text(buffer, "\""),
            other => tracing::warn!("unhandled IME action {:?}", other),
        }
    }

    /// Replace the current selection with `text`, and advance the cursor.
    fn insert_text(&mut self, buffer: &mut T, text: &str) {
        let new_cursor_pos = self.selection.min() + text.len();
        buffer.edit(self.selection.range(), text);
        self.selection = Selection::caret(new_cursor_pos);
        self.scroll_to_selection_end(true);
    }

    fn backspace(&mut self, buffer: &mut T) {
        let to_del = if self.selection.is_caret() {
            let del_start = crate::text::offset_for_delete_backwards(&self.selection, buffer);
            del_start..self.selection.start
        } else {
            self.selection.range()
        };
        self.selection = Selection::caret(to_del.start);
        buffer.edit(to_del, "");
        self.scroll_to_selection_end(true);
    }

    pub fn do_mouse_down(&mut self, point: Point, mods: Modifiers, count: u8) {
        let point = point + Vec2::new(self.alignment_offset, 0.0);
        let pos = self.layout.text_position_for_point(point);
        if mods.shift() {
            self.selection.end = pos;
        } else {
            let sel = self.sel_region_for_pos(pos, count);
            self.selection.start = sel.start;
            self.selection.end = sel.end;
        }
    }

    pub fn do_drag(&mut self, point: Point) {
        let point = point + Vec2::new(self.alignment_offset, 0.0);
        //FIXME: this should behave differently if we were double or triple clicked
        let pos = self.layout.text_position_for_point(point);
        self.selection.end = pos;
        self.scroll_to_selection_end(false);
    }

    /// Returns a line suitable for drawing a standard cursor.
    pub fn cursor_line_for_text_position(&self, pos: usize) -> Line {
        let line = self.layout.cursor_line_for_text_position(pos);
        line + Vec2::new(self.alignment_offset, 0.0)
    }

    fn sel_region_for_pos(&mut self, pos: usize, click_count: u8) -> Range<usize> {
        let text = match self.layout.text() {
            Some(text) => text,
            None => return pos..pos,
        };
        match click_count {
            1 => pos..pos,
            2 => {
                //FIXME: this doesn't handle whitespace correctly
                let word_min = text.prev_word_offset(pos).unwrap_or(0);
                let word_max = text.next_word_offset(pos).unwrap_or_else(|| text.len());
                word_min..word_max
            }
            _ => {
                let line_min = text.preceding_line_break(pos);
                let line_max = text.next_line_break(pos);
                line_min..line_max
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, new_data: &T, env: &Env) {
        self.layout.set_text(new_data.clone());
        if self.layout.needs_rebuild_after_update(ctx) {
            self.selection = self.selection.constrained(new_data);
            ctx.request_layout();
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
    fn selection(&self) -> crate::shell::text::Selection {
        self.inner.borrow().selection.into()
    }

    fn set_selection(&mut self, selection: crate::shell::text::Selection) {
        self.inner.borrow_mut().selection = selection.into();
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

impl<T> Default for SharedTextComponent<T> {
    fn default() -> Self {
        let inner = EditSession {
            layout: TextLayout::new(),
            external_scroll_to: None,
            external_text_change: None,
            selection: Selection::caret(0),
            composition_range: None,
            accepts_newlines: false,
            alignment: TextAlignment::Start,
            alignment_offset: 0.0,
            origin: Point::ZERO,
        };

        SharedTextComponent {
            inner: Arc::new(RefCell::new(inner)),
            lock: Arc::new(Cell::new(ImeLock::None)),
            was_focused_from_click: false,
            cursor_on: false,
            cursor_timer: TimerToken::INVALID,
        }
    }
}
