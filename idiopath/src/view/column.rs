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

use std::{any::Any, marker::PhantomData};

use crate::{
    id::{Id, IdPath},
    view_tuple::ViewTuple,
    widget::WidgetTuple,
};

use super::View;

pub struct Column<T, A, VT: ViewTuple<T, A>> {
    children: VT,
    phantom: PhantomData<(T, A)>,
}

impl<T, A, VT: ViewTuple<T, A>> View<T, A> for Column<T, A, VT>
where
    VT::Elements: WidgetTuple,
{
    type State = VT::State;

    type Element = crate::widget::column::Column<VT::Elements>;

    fn build(&self, id_path: &mut IdPath) -> (Id, Self::State, Self::Element) {
        let id = Id::next();
        id_path.push(id);
        let (state, elements) = self.children.build(id_path);
        id_path.pop();
        let column = crate::widget::column::Column::new(elements);
        (id, state, column)
    }

    fn rebuild(
        &self,
        id_path: &mut IdPath,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) {
        id_path.push(*id);
        self.children
            .rebuild(id_path, &prev.children, state, element.children_mut());
        id_path.pop();
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> A {
        self.children.event(id_path, state, event, app_state)
    }
}
