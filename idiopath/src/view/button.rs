// Copyright 2022 The Druid Authors.
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

use std::any::Any;

use crate::id::Id;

use super::{Cx, View};

pub struct Button<T, A> {
    label: String,
    // consider not boxing
    callback: Box<dyn Fn(&mut T) -> A>,
}

impl<T, A> Button<T, A> {
    pub fn new(label: impl Into<String>, clicked: impl Fn(&mut T) -> A + 'static) -> Self {
        Button {
            label: label.into(),
            callback: Box::new(clicked),
        }
    }
}

impl<T, A> View<T, A> for Button<T, A> {
    type State = ();

    type Element = crate::widget::button::Button;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let id = Id::next();
        cx.push(id);
        let element = crate::widget::button::Button::new(cx.id_path(), self.label.clone());
        cx.pop();
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        prev: &Self,
        _id: &mut crate::id::Id,
        _state: &mut Self::State,
        element: &mut Self::Element,
    ) {
        if prev.label != self.label {
            element.set_label(self.label.clone());
        }
    }

    fn event(
        &self,
        _id_path: &[crate::id::Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        app_state: &mut T,
    ) -> A {
        (self.callback)(app_state)
    }
}
