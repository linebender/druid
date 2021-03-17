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

//! A textbox that that parses and validates data.

use tracing::instrument;

use super::TextBox;
use crate::text::{
    format::{Formatter, ValidationError},
    Selection, TextComponent,
};
use crate::widget::prelude::*;
use crate::{Data, Selector};

const BEGIN_EDITING: Selector = Selector::new("druid.builtin.textbox-begin-editing");
const COMPLETE_EDITING: Selector = Selector::new("druid.builtin.textbox-complete-editing");

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

impl<T: Data> ValueTextBox<T> {
    /// Create a new `ValueTextBox` from a normal [`TextBox`] and a [`Formatter`].
    ///
    /// [`TextBox`]: crate::widget::TextBox
    /// [`Formatter`]: crate::text::format::Formatter
    pub fn new(mut inner: TextBox<String>, formatter: impl Formatter<T> + 'static) -> Self {
        inner.text_mut().borrow_mut().send_notification_on_return = true;
        inner.text_mut().borrow_mut().send_notification_on_cancel = true;
        inner.handles_tab_notifications = false;
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

    fn complete(&mut self, ctx: &mut EventCtx, data: &mut T) -> bool {
        match self.formatter.value(&self.buffer) {
            Ok(new_data) => {
                *data = new_data;
                self.buffer = self.formatter.format(data);
                self.is_editing = false;
                ctx.request_update();
                self.send_event(ctx, TextBoxEvent::Complete);
                true
            }
            Err(err) => {
                if self.inner.text().can_write() {
                    if let Some(inval) = self
                        .inner
                        .text_mut()
                        .borrow_mut()
                        .set_selection(Selection::new(0, self.buffer.len()))
                    {
                        ctx.invalidate_text_input(inval);
                    }
                }
                self.send_event(ctx, TextBoxEvent::Invalid(err));
                // our content isn't valid
                // ideally we would flash the background or something
                false
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

impl<T: Data + std::fmt::Debug> Widget<T> for ValueTextBox<T> {
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
            let pre_sel = if self.inner.text().can_read() {
                Some(self.inner.text().borrow().selection())
            } else {
                None
            };
            match event {
                // this is caused by an external focus change, like the mouse being clicked
                // elsewhere.
                Event::Command(cmd) if cmd.is(COMPLETE_EDITING) => {
                    if !self.complete(ctx, data) {
                        self.cancel(ctx, data);
                    }
                    return;
                }
                Event::Notification(cmd) if cmd.is(TextComponent::TAB) => {
                    ctx.set_handled();
                    ctx.request_paint();
                    if self.complete(ctx, data) {
                        ctx.focus_next();
                    }
                    return;
                }
                Event::Notification(cmd) if cmd.is(TextComponent::BACKTAB) => {
                    ctx.request_paint();
                    ctx.set_handled();
                    if self.complete(ctx, data) {
                        ctx.focus_prev();
                    }
                    return;
                }
                Event::Notification(cmd) if cmd.is(TextComponent::RETURN) => {
                    ctx.set_handled();
                    if self.complete(ctx, data) {
                        ctx.resign_focus();
                    }
                    return;
                }
                Event::Notification(cmd) if cmd.is(TextComponent::CANCEL) => {
                    ctx.set_handled();
                    self.cancel(ctx, data);
                    return;
                }
                event => {
                    self.inner.event(ctx, event, &mut self.buffer, env);
                }
            }
            // if an edit occured, validate it with the formatter
            // notifications can arrive before update, so we always ignore them
            if !matches!(event, Event::Notification(_)) && self.buffer != self.old_buffer {
                let mut validation = self
                    .formatter
                    .validate_partial_input(&self.buffer, &self.inner.text().borrow().selection());

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
                        (None, true) if pre_sel.is_some() => pre_sel,
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
        // if we *aren't* editing:
        } else {
            if let Event::MouseDown(_) = event {
                self.begin(ctx, data);
            }
            self.inner.event(ctx, event, &mut self.buffer, env);
        }
    }

    #[instrument(
        name = "ValueTextBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::WidgetAdded => {
                self.buffer = self.formatter.format(data);
                self.old_buffer = self.buffer.clone();
            }
            LifeCycle::FocusChanged(true) if !self.is_editing => {
                ctx.submit_command(BEGIN_EDITING.to(ctx.widget_id()));
            }
            LifeCycle::FocusChanged(false) => {
                ctx.submit_command(COMPLETE_EDITING.to(ctx.widget_id()));
            }
            _ => (),
        }
        self.inner.lifecycle(ctx, event, &self.buffer, env);
    }

    #[instrument(
        name = "ValueTextBox",
        level = "trace",
        skip(self, ctx, old, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old: &T, data: &T, env: &Env) {
        if let Some(sel) = self.force_selection.take() {
            if self.inner.text().can_write() {
                if let Some(change) = self.inner.text_mut().borrow_mut().set_selection(sel) {
                    ctx.invalidate_text_input(change);
                }
            }
        }
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
            if !old.same(data) {
                // we aren't editing and data changed
                let new_text = self.formatter.format(data);
                // it's possible for different data inputs to produce the same formatted
                // output, in which case we would overwrite our actual previous data
                if !new_text.same(&self.buffer) {
                    self.old_buffer = std::mem::replace(&mut self.buffer, new_text);
                }
            }

            if !self.old_buffer.same(&self.buffer) {
                // inner widget handles calling request_layout, as needed
                self.inner.update(ctx, &self.old_buffer, &self.buffer, env);
                self.old_buffer = self.buffer.clone();
            } else if ctx.env_changed() {
                self.inner.update(ctx, &self.buffer, &self.buffer, env);
            }
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
