// Copyright 2018 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A textbox widget.

use std::time::Duration;
use tracing::{instrument, trace};

use crate::contexts::ChangeCtx;
use crate::debug_state::DebugState;
use crate::kurbo::Insets;
use crate::piet::TextLayout as _;
use crate::text::{
    EditableText, ImeInvalidation, Selection, TextComponent, TextLayout, TextStorage,
};
use crate::widget::prelude::*;
use crate::widget::{Padding, Scroll, WidgetWrapper};
use crate::{
    theme, ArcStr, Color, Command, FontDescriptor, HotKey, KeyEvent, KeyOrValue, Point, Rect,
    SysMods, TextAlignment, TimerToken, Vec2,
};

use super::LabelText;

const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);
const MAC_OR_LINUX_OR_BSD: bool = cfg!(any(
    target_os = "freebsd",
    target_os = "macos",
    target_os = "linux",
    target_os = "openbsd"
));

/// When we scroll after editing or movement, we show a little extra of the document.
const SCROLL_TO_INSETS: Insets = Insets::uniform_xy(40.0, 0.0);

/// A widget that allows user text input.
///
/// # Editing values
///
/// If the text you are editing represents a value of some other type, such
/// as a number, you should use a [`ValueTextBox`] and an appropriate
/// [`Formatter`]. You can create a [`ValueTextBox`] by passing the appropriate
/// [`Formatter`] to [`TextBox::with_formatter`].
///
/// [`Formatter`]: crate::text::Formatter
/// [`ValueTextBox`]: super::ValueTextBox
pub struct TextBox<T> {
    placeholder_text: LabelText<T>,
    placeholder_layout: TextLayout<ArcStr>,
    inner: Scroll<T, Padding<T, TextComponent<T>>>,
    scroll_to_selection_after_layout: bool,
    multiline: bool,
    /// true if a click event caused us to gain focus.
    ///
    /// On macOS, if focus happens via click then we set the selection based
    /// on the click position; if focus happens automatically (e.g. on tab)
    /// then we select our entire contents.
    was_focused_from_click: bool,
    cursor_on: bool,
    cursor_timer: TimerToken,
    /// if `true` (the default), this textbox will attempt to change focus on tab.
    ///
    /// You can override this in a controller if you want to customize tab
    /// behaviour.
    pub handles_tab_notifications: bool,
    text_pos: Point,
}

impl<T: EditableText + TextStorage> TextBox<T> {
    /// Create a new TextBox widget.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::TextBox;
    /// use druid::{ WidgetExt, Data, Lens };
    ///
    /// #[derive(Clone, Data, Lens)]
    /// struct AppState {
    ///     name: String,
    /// }
    ///
    /// let _ = TextBox::new()
    ///     .with_placeholder("placeholder text")
    ///     .lens(AppState::name);
    /// ```
    pub fn new() -> Self {
        let placeholder_text = ArcStr::from("");
        let mut placeholder_layout = TextLayout::new();
        placeholder_layout.set_text_color(theme::PLACEHOLDER_COLOR);
        placeholder_layout.set_text(placeholder_text.clone());

        let mut scroll = Scroll::new(Padding::new(
            theme::TEXTBOX_INSETS,
            TextComponent::default(),
        ))
        .content_must_fill(true);
        scroll.set_enabled_scrollbars(crate::scroll_component::ScrollbarsEnabled::None);
        Self {
            inner: scroll,
            scroll_to_selection_after_layout: false,
            placeholder_text: placeholder_text.into(),
            placeholder_layout,
            multiline: false,
            was_focused_from_click: false,
            cursor_on: false,
            cursor_timer: TimerToken::INVALID,
            handles_tab_notifications: true,
            text_pos: Point::ZERO,
        }
    }

    /// Create a new multi-line `TextBox`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// let multiline = TextBox::multiline()
    ///     .lens(AppState::name);
    /// ```
    pub fn multiline() -> Self {
        let mut this = TextBox::new();
        this.inner
            .set_enabled_scrollbars(crate::scroll_component::ScrollbarsEnabled::Both);
        this.text_mut().borrow_mut().set_accepts_newlines(true);
        this.inner.set_horizontal_scroll_enabled(false);
        this.multiline = true;
        this
    }

    /// If `true` (and this is a [`multiline`] text box) lines will be wrapped
    /// at the maximum layout width.
    ///
    /// If `false`, lines will not be wrapped, and horizontal scrolling will
    /// be enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// //will scroll horizontally
    /// let scroll_text_box = TextBox::new()
    ///     .with_line_wrapping(false)
    ///     .lens(AppState::name);
    ///
    /// //will wrap only for a single line
    /// let wrap_text_box = TextBox::new()
    ///     .with_line_wrapping(true)
    ///     .lens(AppState::name);
    ///
    /// //will scroll as well as having multiple lines
    /// let scroll_multi_line_text_box = TextBox::multiline()
    ///     .with_line_wrapping(false)
    ///     .lens(AppState::name);
    ///
    /// //will wrap for each line
    /// let wrap_multi_line_text_box = TextBox::multiline()
    ///     .with_line_wrapping(true) // this is default and can be removed for the same result
    ///     .lens(AppState::name);
    ///
    /// ```
    /// [`multiline`]: TextBox::multiline
    pub fn with_line_wrapping(mut self, wrap_lines: bool) -> Self {
        self.inner.set_horizontal_scroll_enabled(!wrap_lines);
        self
    }
}

impl<T> TextBox<T> {
    /// Builder-style method for setting the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// let text_box = TextBox::new()
    ///     .with_text_size(14.)
    ///     .lens(AppState::name);
    /// ```
    ///
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// use druid::Key;
    ///
    /// const FONT_SIZE : Key<f64> = Key::new("font-size");
    ///
    /// let text_box = TextBox::new()
    ///     .with_text_size(FONT_SIZE)
    ///     .lens(AppState::name);
    /// ```
    /// [`Key<f64>`]: crate::Key
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
    /// # Examples
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// use druid::TextAlignment;
    ///
    /// let text_box = TextBox::new()
    ///     .with_text_alignment(TextAlignment::Center)
    ///     .lens(AppState::name);
    /// ```
    ///
    /// [`multiline`]: TextBox::multiline
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.set_text_alignment(alignment);
        self
    }

    /// Builder-style method for setting the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// use druid::{ FontDescriptor, FontFamily, Key };
    ///
    /// const FONT : Key<FontDescriptor> = Key::new("font");
    ///
    /// let text_box = TextBox::new()
    ///     .with_font(FontDescriptor::new(FontFamily::MONOSPACE))
    ///     .lens(AppState::name);
    ///
    /// let text_box = TextBox::new()
    ///     .with_font(FONT)
    ///     .lens(AppState::name);
    /// ```
    ///
    ///
    /// [`Key<FontDescriptor>`]: crate::Key
    pub fn with_font(mut self, font: impl Into<KeyOrValue<FontDescriptor>>) -> Self {
        self.set_font(font);
        self
    }

    /// Builder-style method for setting the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    /// # Examples
    /// ```
    /// # use druid::widget::TextBox;
    /// # use druid::{ WidgetExt, Data, Lens };
    /// #
    /// # #[derive(Clone, Data, Lens)]
    /// # struct AppState {
    /// #     name: String,
    /// # }
    /// use druid::{ Color, Key };
    ///
    /// const COLOR : Key<Color> = Key::new("color");
    ///
    /// let text_box = TextBox::new()
    ///     .with_text_color(Color::RED)
    ///     .lens(AppState::name);
    ///
    /// let text_box = TextBox::new()
    ///     .with_text_color(COLOR)
    ///     .lens(AppState::name);
    /// ```
    ///
    /// [`Key<Color>`]: crate::Key
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.set_text_color(color);
        self
    }

    /// Set the text size.
    ///
    /// The argument can be either an `f64` or a [`Key<f64>`].
    ///
    /// [`Key<f64>`]: crate::Key
    pub fn set_text_size(&mut self, size: impl Into<KeyOrValue<f64>>) {
        if !self.text().can_write() {
            tracing::warn!("set_text_size called with IME lock held.");
            return;
        }

        let size = size.into();
        self.text_mut()
            .borrow_mut()
            .layout
            .set_text_size(size.clone());
        self.placeholder_layout.set_text_size(size);
    }

    /// Set the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Key<FontDescriptor>`]: crate::Key
    pub fn set_font(&mut self, font: impl Into<KeyOrValue<FontDescriptor>>) {
        if !self.text().can_write() {
            tracing::warn!("set_font called with IME lock held.");
            return;
        }
        let font = font.into();
        self.text_mut().borrow_mut().layout.set_font(font.clone());
        self.placeholder_layout.set_font(font);
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
    /// [`multiline`]: TextBox::multiline
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        if !self.text().can_write() {
            tracing::warn!("set_text_alignment called with IME lock held.");
            return;
        }
        self.text_mut().borrow_mut().set_text_alignment(alignment);
    }

    /// Set the text color.
    ///
    /// The argument can be either a `Color` or a [`Key<Color>`].
    ///
    /// If you change this property, you are responsible for calling
    /// [`request_layout`] to ensure the label is updated.
    ///
    /// [`request_layout`]: EventCtx::request_layout
    /// [`Key<Color>`]: crate::Key
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        if !self.text().can_write() {
            tracing::warn!("set_text_color called with IME lock held.");
            return;
        }
        self.text_mut().borrow_mut().layout.set_text_color(color);
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

impl<T: Data> TextBox<T> {
    /// Builder-style method to set the `TextBox`'s placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<LabelText<T>>) -> Self {
        self.set_placeholder(placeholder);
        self
    }

    /// Set the `TextBox`'s placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<LabelText<T>>) {
        self.placeholder_text = placeholder.into();
        self.placeholder_layout
            .set_text(self.placeholder_text.display_text());
    }
}

impl<T> TextBox<T> {
    /// An immutable reference to the inner [`TextComponent`].
    ///
    /// Using this correctly is difficult; please see the [`TextComponent`]
    /// docs for more information.
    pub fn text(&self) -> &TextComponent<T> {
        self.inner.child().wrapped()
    }

    /// A mutable reference to the inner [`TextComponent`].
    ///
    /// Using this correctly is difficult; please see the [`TextComponent`]
    /// docs for more information.
    pub fn text_mut(&mut self) -> &mut TextComponent<T> {
        self.inner.child_mut().wrapped_mut()
    }

    fn reset_cursor_blink(&mut self, token: TimerToken) {
        self.cursor_on = true;
        self.cursor_timer = token;
    }

    fn should_draw_cursor(&self) -> bool {
        if cfg!(target_os = "macos") && self.text().can_read() {
            self.cursor_on && self.text().borrow().selection().is_caret()
        } else {
            self.cursor_on
        }
    }
}

impl<T: TextStorage + EditableText> TextBox<T> {
    fn rect_for_selection_end(&self) -> Rect {
        let text = self.text().borrow();
        let layout = text.layout.layout().unwrap();

        let hit = layout.hit_test_text_position(text.selection().active);
        let line = layout.line_metric(hit.line).unwrap();
        let y0 = line.y_offset;
        let y1 = y0 + line.height;
        let x = hit.point.x;

        Rect::new(x, y0, x, y1)
    }

    fn scroll_to_selection_end<C: ChangeCtx>(&mut self, ctx: &mut C) {
        let rect = self.rect_for_selection_end();
        let view_rect = self.inner.viewport_rect();
        let is_visible =
            view_rect.contains(rect.origin()) && view_rect.contains(Point::new(rect.x1, rect.y1));
        if !is_visible {
            self.inner.scroll_to(ctx, rect + SCROLL_TO_INSETS);
        }
    }

    /// These commands may be supplied by menus; but if they aren't, we
    /// inject them again, here.
    fn fallback_do_builtin_command(
        &mut self,
        ctx: &mut EventCtx,
        key: &KeyEvent,
    ) -> Option<Command> {
        use crate::commands as sys;
        let our_id = ctx.widget_id();
        match key {
            key if HotKey::new(SysMods::Cmd, "c").matches(key) => Some(sys::COPY.to(our_id)),
            key if HotKey::new(SysMods::Cmd, "x").matches(key) => Some(sys::CUT.to(our_id)),
            // we have to send paste to the window, in order to get it converted into the `Paste`
            // event
            key if HotKey::new(SysMods::Cmd, "v").matches(key) => {
                Some(sys::PASTE.to(ctx.window_id()))
            }
            key if HotKey::new(SysMods::Cmd, "z").matches(key) => Some(sys::UNDO.to(our_id)),
            key if HotKey::new(SysMods::CmdShift, "Z").matches(key) && !cfg!(windows) => {
                Some(sys::REDO.to(our_id))
            }
            key if HotKey::new(SysMods::Cmd, "y").matches(key) && cfg!(windows) => {
                Some(sys::REDO.to(our_id))
            }
            key if HotKey::new(SysMods::Cmd, "a").matches(key) => Some(sys::SELECT_ALL.to(our_id)),
            _ => None,
        }
    }
}

impl<T: TextStorage + EditableText> Widget<T> for TextBox<T> {
    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Notification(cmd) => match cmd {
                cmd if cmd.is(TextComponent::SCROLL_TO) => {
                    let after_edit = *cmd.get(TextComponent::SCROLL_TO).unwrap_or(&false);
                    if after_edit {
                        ctx.request_layout();
                        self.scroll_to_selection_after_layout = true;
                    } else {
                        self.scroll_to_selection_end(ctx);
                    }
                    ctx.set_handled();
                    ctx.request_paint();
                }
                cmd if cmd.is(TextComponent::TAB) && self.handles_tab_notifications => {
                    ctx.focus_next();
                    ctx.request_paint();
                    ctx.set_handled();
                }
                cmd if cmd.is(TextComponent::BACKTAB) && self.handles_tab_notifications => {
                    ctx.focus_prev();
                    ctx.request_paint();
                    ctx.set_handled();
                }
                cmd if cmd.is(TextComponent::CANCEL) => {
                    ctx.resign_focus();
                    ctx.request_paint();
                    ctx.set_handled();
                }
                _ => (),
            },
            Event::KeyDown(key) if !self.text().is_composing() => {
                if let Some(cmd) = self.fallback_do_builtin_command(ctx, key) {
                    ctx.submit_command(cmd);
                    ctx.set_handled();
                }
            }
            Event::MouseDown(mouse) if self.text().can_write() => {
                if !ctx.is_disabled() {
                    if !mouse.focus {
                        ctx.request_focus();
                        self.was_focused_from_click = true;
                        self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                    } else {
                        ctx.set_handled();
                    }
                }
            }
            Event::Timer(id) => {
                if !ctx.is_disabled() {
                    if *id == self.cursor_timer && ctx.has_focus() {
                        self.cursor_on = !self.cursor_on;
                        ctx.request_paint();
                        self.cursor_timer = ctx.request_timer(CURSOR_BLINK_DURATION);
                    }
                } else if self.cursor_on {
                    self.cursor_on = false;
                    ctx.request_paint();
                }
            }
            Event::ImeStateChange => {
                self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
            }
            Event::Command(ref cmd)
                if !self.text().is_composing()
                    && ctx.is_focused()
                    && cmd.is(crate::commands::COPY) =>
            {
                self.text().borrow().set_clipboard();
                ctx.set_handled();
            }
            Event::Command(cmd)
                if !self.text().is_composing()
                    && ctx.is_focused()
                    && cmd.is(crate::commands::CUT) =>
            {
                if self.text().borrow().set_clipboard() {
                    let inval = self.text_mut().borrow_mut().insert_text(data, "");
                    ctx.invalidate_text_input(inval);
                }
                ctx.set_handled();
            }
            Event::Command(cmd)
                if !self.text().is_composing()
                    && ctx.is_focused()
                    && cmd.is(crate::commands::SELECT_ALL) =>
            {
                if let Some(inval) = self
                    .text_mut()
                    .borrow_mut()
                    .set_selection(Selection::new(0, data.as_str().len()))
                {
                    ctx.request_paint();
                    ctx.invalidate_text_input(inval);
                }
                ctx.set_handled();
            }
            Event::Paste(ref item) if self.text().can_write() => {
                if let Some(string) = item.get_string() {
                    let text = if self.multiline {
                        &string
                    } else {
                        string.lines().next().unwrap_or("")
                    };
                    if !text.is_empty() {
                        let inval = self.text_mut().borrow_mut().insert_text(data, text);
                        ctx.invalidate_text_input(inval);
                    }
                }
            }
            _ => (),
        }
        self.inner.event(ctx, event, data, env)
    }

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                if matches!(event, LifeCycle::WidgetAdded) {
                    self.placeholder_text.resolve(data, env);
                }
                ctx.register_text_input(self.text().input_handler());
            }
            LifeCycle::BuildFocusChain => {
                //TODO: make this a configurable option? maybe?
                ctx.register_for_focus();
            }
            LifeCycle::FocusChanged(true) => {
                if self.text().can_write() && !self.multiline && !self.was_focused_from_click {
                    let selection = Selection::new(0, data.len());
                    let _ = self.text_mut().borrow_mut().set_selection(selection);
                    ctx.invalidate_text_input(ImeInvalidation::SelectionChanged);
                }
                self.text_mut().has_focus = true;
                self.reset_cursor_blink(ctx.request_timer(CURSOR_BLINK_DURATION));
                self.was_focused_from_click = false;
                ctx.request_paint();
                ctx.scroll_to_view();
            }
            LifeCycle::FocusChanged(false) => {
                if self.text().can_write() && MAC_OR_LINUX_OR_BSD && !self.multiline {
                    let selection = self.text().borrow().selection();
                    let selection = Selection::new(selection.active, selection.active);
                    let _ = self.text_mut().borrow_mut().set_selection(selection);
                    ctx.invalidate_text_input(ImeInvalidation::SelectionChanged);
                }
                self.text_mut().has_focus = false;
                if !self.multiline {
                    self.inner.scroll_to(ctx, Rect::ZERO);
                }
                self.cursor_timer = TimerToken::INVALID;
                self.was_focused_from_click = false;
                ctx.request_paint();
            }
            _ => (),
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, old, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, old: &T, data: &T, env: &Env) {
        let placeholder_changed = self.placeholder_text.resolve(data, env);
        if placeholder_changed {
            let new_text = self.placeholder_text.display_text();
            self.placeholder_layout.set_text(new_text);
        }

        self.inner.update(ctx, old, data, env);
        if placeholder_changed
            || (ctx.env_changed() && self.placeholder_layout.needs_rebuild_after_update(ctx))
        {
            ctx.request_layout();
        }
        if self.text().can_write() {
            if let Some(ime_invalidation) = self.text_mut().borrow_mut().pending_ime_invalidation()
            {
                ctx.invalidate_text_input(ime_invalidation);
            }
        }
    }

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if !self.text().can_write() {
            tracing::warn!("Widget::layout called with outstanding IME lock.");
        }
        let min_width = env.get(theme::WIDE_WIDGET_WIDTH);
        let textbox_insets = env.get(theme::TEXTBOX_INSETS);

        self.placeholder_layout.rebuild_if_needed(ctx.text(), env);
        let min_size = bc.constrain((min_width, 0.0));
        let child_bc = BoxConstraints::new(min_size, bc.max());

        let size = self.inner.layout(ctx, &child_bc, data, env);

        let text_metrics = if !self.text().can_read() || data.is_empty() {
            self.placeholder_layout.layout_metrics()
        } else {
            self.text().borrow().layout.layout_metrics()
        };

        let layout_baseline = text_metrics.size.height - text_metrics.first_baseline;
        let baseline_off = layout_baseline
            - (self.inner.child_size().height - self.inner.viewport_rect().height())
            + textbox_insets.y1;
        ctx.set_baseline_offset(baseline_off);
        if self.scroll_to_selection_after_layout {
            self.scroll_to_selection_end(ctx);
            self.scroll_to_selection_after_layout = false;
        }

        trace!(
            "Computed layout: size={}, baseline_offset={:?}",
            size,
            baseline_off
        );
        size
    }

    #[instrument(name = "TextBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if !self.text().can_read() {
            tracing::warn!("Widget::paint called with outstanding IME lock, skipping");
            return;
        }
        let size = ctx.size();
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let cursor_color = env.get(theme::CURSOR_COLOR);
        let border_width = env.get(theme::TEXTBOX_BORDER_WIDTH);
        let textbox_insets = env.get(theme::TEXTBOX_INSETS);

        let is_focused = ctx.is_focused();

        let border_color = if is_focused {
            env.get(theme::PRIMARY_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        // Paint the background
        let clip_rect = size
            .to_rect()
            .inset(-border_width / 2.0)
            .to_rounded_rect(env.get(theme::TEXTBOX_BORDER_RADIUS));

        ctx.fill(clip_rect, &background_color);

        if !data.is_empty() {
            self.inner.paint(ctx, data, env);
        } else {
            let text_width = self.placeholder_layout.layout_metrics().size.width;
            let extra_width = (size.width - text_width - textbox_insets.x_value()).max(0.);
            let alignment = self.text().borrow().text_alignment();
            // alignment is only used for single-line text boxes
            let x_offset = if self.multiline {
                0.0
            } else {
                x_offset_for_extra_width(alignment, extra_width)
            };

            // clip when we draw the placeholder, since it isn't in a clipbox
            ctx.with_save(|ctx| {
                ctx.clip(clip_rect);
                self.placeholder_layout
                    .draw(ctx, (textbox_insets.x0 + x_offset, textbox_insets.y0));
            })
        }

        // Paint the cursor if focused and there's no selection
        if is_focused && self.should_draw_cursor() {
            // if there's no data, we always draw the cursor based on
            // our alignment.
            let cursor_pos = self.text().borrow().selection().active;
            let cursor_line = self
                .text()
                .borrow()
                .cursor_line_for_text_position(cursor_pos);

            let padding_offset = Vec2::new(textbox_insets.x0, textbox_insets.y0);

            let mut cursor = if data.is_empty() {
                cursor_line + padding_offset
            } else {
                cursor_line + padding_offset - self.inner.offset()
            };

            // Snap the cursor to the pixel grid so it stays sharp.
            cursor.p0.x = cursor.p0.x.trunc() + 0.5;
            cursor.p1.x = cursor.p0.x;

            ctx.with_save(|ctx| {
                ctx.clip(clip_rect);
                ctx.stroke(cursor, &cursor_color, 1.);
            })
        }

        // Paint the border
        ctx.stroke(clip_rect, &border_color, border_width);
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let text = data.slice(0..data.len()).unwrap_or_default();
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: text.to_string(),
            ..Default::default()
        }
    }
}

impl<T: TextStorage + EditableText> Default for TextBox<T> {
    fn default() -> Self {
        TextBox::new()
    }
}

fn x_offset_for_extra_width(alignment: TextAlignment, extra_width: f64) -> f64 {
    match alignment {
        TextAlignment::Start | TextAlignment::Justified => 0.0,
        TextAlignment::End => extra_width,
        TextAlignment::Center => extra_width / 2.0,
    }
}
