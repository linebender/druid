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

use crate::{event::EventResult, id::Id};

use super::{Cx, View};

pub struct Memoize<D, F> {
    data: D,
    child_cb: F,
}

pub struct MemoizeState<T, A, V: View<T, A>> {
    view: V,
    view_state: V::State,
    dirty: bool,
}

impl<D, V, F: Fn(&D) -> V> Memoize<D, F> {
    pub fn new(data: D, child_cb: F) -> Self {
        Memoize { data, child_cb }
    }
}

impl<T, A, D: PartialEq + Clone + 'static, V: View<T, A>, F: Fn(&D) -> V> View<T, A>
    for Memoize<D, F>
{
    type State = MemoizeState<T, A, V>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let view = (self.child_cb)(&self.data);
        let (id, view_state, element) = view.build(cx);
        let memoize_state = MemoizeState {
            view,
            view_state,
            dirty: false,
        };
        (id, memoize_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        if std::mem::take(&mut state.dirty) || prev.data != self.data {
            let view = (self.child_cb)(&self.data);
            let changed = view.rebuild(cx, &state.view, id, &mut state.view_state, element);
            state.view = view;
            changed
        } else {
            false
        }
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        let r = state
            .view
            .event(id_path, &mut state.view_state, event, app_state);
        if matches!(r, EventResult::RequestRebuild) {
            state.dirty = true;
        }
        r
    }
}
