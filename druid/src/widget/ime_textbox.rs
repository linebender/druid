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

use crate::kurbo::Insets;
use crate::piet::TextLayout as _;
use crate::text::{EditableText, SharedTextComponent, TextLayout, TextStorage};
use crate::widget::prelude::*;
use crate::widget::{Padding, Scroll};
use crate::{theme, Color, FontDescriptor, KeyOrValue, Point, Rect, TextAlignment, Vec2};

const TEXTBOX_INSETS: Insets = Insets::new(4.0, 2.0, 4.0, 2.0);

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
pub struct ImeTextBox<T> {
    placeholder: TextLayout<String>,
    inner: Padding<T, Scroll<T, SharedTextComponent<T>>>,
    scroll_to_selection_after_layout: bool,
    multiline: bool,
    wrap_lines: bool,
    text_pos: Point,
}

impl<T: EditableText + TextStorage> ImeTextBox<T> {
    /// Create a new TextBox widget.
    pub fn new() -> Self {
        let mut placeholder = TextLayout::from_text("");
        placeholder.set_text_color(theme::PLACEHOLDER_COLOR);
        let mut scroll = Scroll::new(SharedTextComponent::default()).content_must_fill(true);
        scroll.set_enabled_scrollbars(crate::scroll_component::ScrollbarsEnabled::None);
        Self {
            inner: Padding::new(TEXTBOX_INSETS, scroll),
            scroll_to_selection_after_layout: false,
            placeholder,
            multiline: false,
            wrap_lines: false,
            text_pos: Point::ZERO,
        }
    }

    /// Create a new multi-line `TextBox`.
    pub fn multiline() -> Self {
        let mut this = ImeTextBox::new();
        this.inner
            .child_mut()
            .set_enabled_scrollbars(crate::scroll_component::ScrollbarsEnabled::Both);
        this.text_mut().borrow_mut().set_accepts_newlines(true);
        this.inner.child_mut().set_horizontal_scroll_enabled(false);
        this.multiline = true;
        this
    }

    /// If `true` (and this is a [`multiline`] text box) lines will be wrapped
    /// at the maximum layout width.
    ///
    /// If `false`, lines will not be wrapped, and horizontal scrolling will
    /// be enabled.
    pub fn with_line_wrapping(mut self, wrap_lines: bool) -> Self {
        self.wrap_lines = wrap_lines;
        self.inner
            .child_mut()
            .set_horizontal_scroll_enabled(!wrap_lines);
        self
    }
}

impl<T> ImeTextBox<T> {
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
        if !self.text().can_write() {
            tracing::warn!("set_text_size called with IME lock held.");
            return;
        }

        let size = size.into();
        self.text_mut()
            .borrow_mut()
            .layout
            .set_text_size(size.clone());
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
        if !self.text().can_write() {
            tracing::warn!("set_font called with IME lock held.");
            return;
        }
        let font = font.into();
        self.text_mut().borrow_mut().layout.set_font(font.clone());
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
    /// [`request_layout`]: ../struct.EventCtx.html#method.request_layout
    /// [`Key<Color>`]: ../struct.Key.html
    pub fn set_text_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        if !self.text().can_write() {
            tracing::warn!("set_text_color calld with IME lock held.");
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

impl<T> ImeTextBox<T> {
    ///// Set the textbox's selection.
    //pub fn set_selection(&mut self, selection: Selection) {
    //self.editor.set_selection(selection);
    //}

    ///// Set the text and force the editor to update.
    /////
    ///// This should be rarely needed; the main use-case would be if you need
    ///// to manually set the text and then immediately do hit-testing or other
    ///// tasks that rely on having an up-to-date text layout.
    //pub fn force_rebuild(&mut self, text: T, factory: &mut PietText, env: &Env) {
    //self.editor.set_text(text);
    //self.editor.rebuild_if_needed(factory, env);
    //}
}

impl<T> ImeTextBox<T> {
    fn text(&self) -> &SharedTextComponent<T> {
        self.inner.child().child()
    }

    fn text_mut(&mut self) -> &mut SharedTextComponent<T> {
        self.inner.child_mut().child_mut()
    }
}

impl<T: TextStorage + EditableText> ImeTextBox<T> {
    fn rect_for_selection_end(&self) -> Rect {
        let selection_end = self.text().borrow().selection().end;
        let hit = self
            .text()
            .borrow()
            .layout
            .layout()
            .unwrap()
            .hit_test_text_position(selection_end);
        let line = self
            .text()
            .borrow()
            .layout
            .layout()
            .unwrap()
            .line_metric(hit.line)
            .unwrap();
        let y0 = line.y_offset;
        let y1 = y0 + line.height;
        let x = hit.point.x;

        Rect::new(x, y0, x, y1)
    }

    fn scroll_to_selection_end(&mut self) {
        let rect = self.rect_for_selection_end();
        let view_rect = self.inner.child().viewport_rect();
        let is_visible =
            view_rect.contains(rect.origin()) && view_rect.contains(Point::new(rect.x1, rect.y1));
        if !is_visible {
            self.inner.child_mut().scroll_to(rect + SCROLL_TO_INSETS);
        }
    }
}

impl<T: TextStorage + EditableText> Widget<T> for ImeTextBox<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Notification(cmd) = event {
            match cmd {
                cmd if cmd.is(SharedTextComponent::REDRAW_CURSOR) => {
                    ctx.request_paint();
                    ctx.set_handled();
                }
                cmd if cmd.is(SharedTextComponent::SCROLL_TO) => {
                    let after_edit = *cmd.get(SharedTextComponent::SCROLL_TO).unwrap_or(&false);
                    if after_edit {
                        ctx.request_layout();
                        self.scroll_to_selection_after_layout = true;
                    } else {
                        self.scroll_to_selection_end();
                    }
                    ctx.set_handled();
                    ctx.request_paint();
                }
                _ => (),
            }
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
        match event {
            LifeCycle::WidgetAdded => {
                ctx.register_for_focus();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old, data, env);
        if ctx.env_changed() && self.placeholder.needs_rebuild_after_update(ctx) {
            ctx.request_layout();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if !self.text().can_write() {
            tracing::warn!("Widget::layout called with outstanding IME lock.");
        }
        let min_width = env.get(theme::WIDE_WIDGET_WIDTH);

        self.placeholder.rebuild_if_needed(ctx.text(), env);
        let min_size = bc.constrain((min_width, 0.0));
        let child_bc = BoxConstraints::new(min_size, bc.max());

        let size = self.inner.layout(ctx, &child_bc, data, env);

        let text_metrics = if !self.text().can_read() || data.is_empty() {
            self.placeholder.layout_metrics()
        } else {
            self.text().borrow().layout.layout_metrics()
        };

        let layout_baseline = text_metrics.size.height - text_metrics.first_baseline;
        let baseline_off = layout_baseline
            - (self.inner.child().child_size().height
                - self.inner.child().viewport_rect().height())
            + TEXTBOX_INSETS.y1;
        ctx.set_baseline_offset(baseline_off);
        if self.scroll_to_selection_after_layout {
            self.scroll_to_selection_end();
            self.scroll_to_selection_after_layout = false;
        }

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if !self.text().can_read() {
            tracing::warn!("Widget::paint called with outstanding IME lock, skipping");
            return;
        }
        let size = ctx.size();
        let background_color = env.get(theme::BACKGROUND_LIGHT);
        let cursor_color = env.get(theme::CURSOR_COLOR);
        let border_width = env.get(theme::TEXTBOX_BORDER_WIDTH);

        let is_focused = ctx.has_focus();

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

        if !data.is_empty() {
            self.inner.paint(ctx, data, env);
        } else {
            // clip when we draw the placeholder, since it isn't in a clipbox
            ctx.with_save(|ctx| {
                ctx.clip(clip_rect);
                self.placeholder
                    .draw(ctx, (TEXTBOX_INSETS.x0, TEXTBOX_INSETS.y0));
            })
        }

        // Paint the cursor if focused and there's no selection
        if is_focused && self.text().should_draw_cursor() {
            // if there's no data, we always draw the cursor based on
            // our alignment.
            let cursor_pos = self.text().borrow().selection().end;
            let cursor_line = self
                .text()
                .borrow()
                .cursor_line_for_text_position(cursor_pos);

            let padding_offset = Vec2::new(TEXTBOX_INSETS.x0, TEXTBOX_INSETS.y0);

            let cursor = if data.is_empty() {
                cursor_line + padding_offset
            } else {
                cursor_line + padding_offset - self.inner.child().offset()
            };
            ctx.stroke(cursor, &cursor_color, 1.);
        }

        // Paint the border
        ctx.stroke(clip_rect, &border_color, border_width);
    }
}

impl<T: TextStorage + EditableText> Default for ImeTextBox<T> {
    fn default() -> Self {
        ImeTextBox::new()
    }
}
