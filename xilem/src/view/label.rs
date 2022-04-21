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

use std::{any::Any};

use crate::{event::EventResult, id::Id};

use super::{Cx, View};

pub struct Label {
    label: String,
}

impl Label {
    pub fn new(label: impl Into<String>) -> Self {
        Label {
            label: label.into(),
        }
    }
}

impl<T> View<T, ()> for Label {
    type State = ();

    type Element = crate::widget::label::Label;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx
            .with_new_id(|cx| crate::widget::label::Label::new(self.label.clone()));
        (id, (), element)
    }

    fn rebuild(
            &self,
            cx: &mut super::Cx,
            prev: &Self,
            id: &mut Id,
            state: &mut Self::State,
            element: &mut Self::Element,
        ) -> bool {
        if prev.label != self.label {
            element.set_label(self.label.clone());
            true
        } else {
            false
        }
    }

    fn event(
            &self,
            id_path: &[crate::id::Id],
            state: &mut Self::State,
            event: Box<dyn Any>,
            app_state: &mut T,
        ) -> crate::event::EventResult<()> {
        EventResult::Nop
    }
}
