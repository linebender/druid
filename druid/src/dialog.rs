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

use crate::widget::prelude::*;
use crate::widget::{BackgroundBrush, Button, Flex, Label, LabelText, WidgetExt};
use crate::{Color, Data, ModalDesc};

struct DialogOption<T> {
    text: LabelText<T>,
    action: Box<dyn Fn(&mut EventCtx, &mut T, &Env)>,
}

pub struct DialogDesc<T> {
    text: LabelText<T>,
    options: Vec<DialogOption<T>>,
    background: Option<BackgroundBrush<T>>,
}

impl<T: Data> DialogDesc<T> {
    pub fn new(text: impl Into<LabelText<T>>) -> DialogDesc<T> {
        DialogDesc {
            text: text.into(),
            options: Vec::new(),
            background: None,
        }
    }

    pub fn with_option(
        mut self,
        text: impl Into<LabelText<T>>,
        action: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> Self {
        self.options.push(DialogOption {
            text: text.into(),
            action: Box::new(action),
        });
        self
    }

    pub fn background(mut self, brush: impl Into<BackgroundBrush<T>>) -> Self {
        self.background = Some(brush.into());
        self
    }

    fn into_widget(self) -> impl Widget<T> {
        let mut button_row = Flex::row();
        for opt in self.options {
            // TODO: better distribution of the buttons
            let action = opt.action;
            button_row.add_child(Button::new(opt.text).on_click(move |ctx, data, env| {
                action(ctx, data, env);
                ctx.submit_command(ModalDesc::DISMISS_MODAL, ctx.window_id());
            }));
        }
        let label = Label::new(self.text);
        let col = Flex::column().with_child(label).with_child(button_row);
        col.center().expand().background(
            self.background
                .unwrap_or_else(|| Color::WHITE.with_alpha(0.0).into()),
        )
    }

    pub fn into_modal_desc(self) -> ModalDesc<T> {
        ModalDesc::new(self.into_widget())
    }
}
