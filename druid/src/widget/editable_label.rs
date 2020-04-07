// Copyright 2020 The xi-editor Authors.
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

//! A label that can be edited.
//!
//! This is a bit hacky, and depends on implementation details of other widgets.

use crate::text::EditAction;
use crate::widget::prelude::*;
use crate::widget::{Label, LabelText, TextBox};
use crate::{Command, Data, HotKey, KeyCode, Selector};

// we send this to ourselves if another widget takes focus, in order
// to validate and move out of editing mode
const LOST_FOCUS: Selector = Selector::new("druid.builtin.EditableLabel-lost-focus");

/// A label with text that can be edited.
///
/// Edits are not applied to the data until editing finishes, usually when the
/// user presses <return>. If the new text generates a valid value, it is set;
/// otherwise editing continues.
///
/// Editing can be abandoned by pressing <esc>.
pub struct EditableLabel<T> {
    label: Label<T>,
    buffer: String,
    editing: bool,
    text_box: TextBox,
    on_completion: Box<dyn Fn(&str) -> Option<T>>,
}

impl<T: Data + std::fmt::Display + std::str::FromStr> EditableLabel<T> {
    /// Create a new `EditableLabel` that uses `to_string` to display a value and
    /// `FromStr` to validate the input.
    pub fn parse() -> Self {
        Self::new(|data: &T, _: &_| data.to_string(), |s| s.parse().ok())
    }
}

impl<T: Data> EditableLabel<T> {
    /// Create a new `EditableLabel`.
    ///
    /// The first argument creates a label; it should probably be a dynamic
    /// or localized string.
    ///
    /// The second argument is a closure used to compute the data from the
    /// contents of the string. This is called when the user presses return,
    /// or otherwise tries to navigate away from the label; if it returns
    /// `Some<T>` then that is set as the new data, and the edit ends. If it
    /// returns `None`, then the edit continues.
    pub fn new(
        text: impl Into<LabelText<T>>,
        on_completion: impl Fn(&str) -> Option<T> + 'static,
    ) -> Self {
        EditableLabel {
            label: Label::new(text),
            buffer: String::new(),
            text_box: TextBox::new(),
            editing: false,
            on_completion: Box::new(on_completion),
        }
    }

    fn complete(&mut self, ctx: &mut EventCtx, data: &mut T) {
        if let Some(new) = (self.on_completion)(&self.buffer) {
            *data = new;
            self.editing = false;
            ctx.request_layout();
            ctx.resign_focus();
        } else {
            // don't tab away from here if we're editing
            if !ctx.has_focus() {
                ctx.request_focus();
            }
            ctx.submit_command(
                Command::new(TextBox::PERFORM_EDIT, EditAction::SelectAll),
                ctx.widget_id(),
            );
            // our content isn't valid
            // ideally we would flash the background or something
        }
    }

    fn cancel(&mut self, ctx: &mut EventCtx) {
        self.editing = false;
        ctx.request_layout();
        ctx.resign_focus();
    }

    fn begin(&mut self, ctx: &mut EventCtx) {
        self.editing = true;
        self.buffer = self.label.text().to_string();
        ctx.request_layout();
    }
}

impl<T: Data> Widget<T> for EditableLabel<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.editing {
            match event {
                Event::Command(cmd) if cmd.selector == LOST_FOCUS => {
                    self.complete(ctx, data);
                }
                Event::KeyDown(k_e) if HotKey::new(None, KeyCode::Escape).matches(k_e) => {
                    self.cancel(ctx);
                }
                Event::KeyDown(k_e) if HotKey::new(None, KeyCode::Return).matches(k_e) => {
                    self.complete(ctx, data);
                }
                event => {
                    self.text_box.event(ctx, event, &mut self.buffer, env);
                    ctx.request_paint();
                }
            }
        } else if let Event::MouseDown(_) = event {
            self.begin(ctx);
            self.text_box.event(ctx, event, &mut self.buffer, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.label.lifecycle(ctx, event, data, env);
        self.text_box.lifecycle(ctx, event, &self.buffer, env);

        if let LifeCycle::FocusChanged(focus) = event {
            // if the user focuses elsewhere, we need to reset ourselves
            if !focus {
                ctx.submit_command(LOST_FOCUS, None);
            } else if !self.editing {
                self.editing = true;
                self.buffer = self.label.text().to_string();
                ctx.request_layout();
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        // what should this do? If we're editing, does it cancel our editing? No.
        if !self.editing {
            self.label.update(ctx, old_data, data, env);
        }
        // we don't update the textbox because we don't bother keeping the old
        // data. If the implementation of textbox changes, though, we would break?
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let textbox_size = self.text_box.layout(ctx, bc, &self.buffer, env);
        let label_constraints = BoxConstraints::new(textbox_size, bc.max());
        if self.editing {
            textbox_size
        } else {
            // label should be at least as large as textbox
            self.label.layout(ctx, &label_constraints, data, env)
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.editing {
            self.text_box.paint(ctx, &self.buffer, env);
        } else {
            self.label.paint(ctx, data, env);
        }
    }
}
