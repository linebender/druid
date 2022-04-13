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

use std::{marker::PhantomData, any::Any};

use crate::id::{Id, IdPath};

use super::View;

pub struct UseState<T, A, S, V, FInit, F> {
    f_init: FInit,
    f: F,
    phantom: PhantomData<(T, A, S, V)>,
}

pub struct UseStateState<T, A, S, V: View<T, A>> {
    state: S,
    view: V,
    view_state: V::State,
}

impl<T, A, S, V, FInit: Fn() -> S, F: Fn(&mut S) -> V> UseState<T, A, S, V, FInit, F> {
    pub fn new(f_init: FInit, f: F) -> Self {
        let phantom = Default::default();
        UseState {
            f_init, f, phantom
        }
    }
}

impl<T, A, S, V: View<T, A>, FInit: Fn() -> S, F: Fn(&mut S) -> V> View<T, A> for UseState<T, A, S, V, FInit, F> {
    type State = UseStateState<T, A, S, V>;

    type Element = V::Element;

    fn build(&self, id_path: &mut IdPath) -> (Id, Self::State, Self::Element) {
        let mut state = (self.f_init)();
        let view = (self.f)(&mut state);
        let (id, view_state, element) = view.build(id_path);
        let my_state = UseStateState { state, view, view_state };
        (id, my_state, element)
    }
    fn rebuild(
        &self,
        id_path: &mut IdPath,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) {
        let view = (self.f)(&mut state.state);
        view.rebuild(id_path, &state.view, id, &mut state.view_state, element);
        state.view = view;
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> A {
        state.view.event(id_path, &state.view_state, event, app_state)
    }
}
