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

use std::{any::Any, marker::PhantomData, sync::Arc};

use crate::{event::EventResult, id::Id};

use super::{Cx, View};

/// An implementation of the "use_state" pattern familiar in reactive UI.
///
/// This may not be the final form. In this version, the parent app data
/// is `Rc<T>`, and the child is `(Rc<T>, S)` where S is the local state.
///
/// The first callback creates the initial state (it is called on build but
/// not rebuild). The second callback takes that state as an argument. It
/// is not passed the app state, but since that state is `Rc`, it would be
/// natural to clone it and capture it in a `move` closure.
pub struct UseState<T, A, S, V, FInit, F> {
    f_init: FInit,
    f: F,
    phantom: PhantomData<fn() -> (T, A, S, V)>,
}

pub struct UseStateState<T, A, S, V: View<(Arc<T>, S), A>> {
    state: Option<S>,
    view: V,
    view_state: V::State,
}

impl<T, A, S, V, FInit: Fn() -> S, F: Fn(&mut S) -> V> UseState<T, A, S, V, FInit, F> {
    #[allow(unused)]
    pub fn new(f_init: FInit, f: F) -> Self {
        let phantom = Default::default();
        UseState { f_init, f, phantom }
    }
}

impl<T, A, S, V: View<(Arc<T>, S), A>, FInit: Fn() -> S, F: Fn(&mut S) -> V> View<Arc<T>, A>
    for UseState<T, A, S, V, FInit, F>
where
    S: Send,
    FInit: Send,
    F: Send,
{
    type State = UseStateState<T, A, S, V>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut state = (self.f_init)();
        let view = (self.f)(&mut state);
        let (id, view_state, element) = view.build(cx);
        let my_state = UseStateState {
            state: Some(state),
            view,
            view_state,
        };
        (id, my_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        let view = (self.f)(state.state.as_mut().unwrap());
        let changed = view.rebuild(cx, &state.view, id, &mut state.view_state, element);
        state.view = view;
        changed
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut Arc<T>,
    ) -> EventResult<A> {
        let mut local_state = (app_state.clone(), state.state.take().unwrap());
        let a = state
            .view
            .event(id_path, &mut state.view_state, event, &mut local_state);
        let (local_app_state, my_state) = local_state;
        if !Arc::ptr_eq(app_state, &local_app_state) {
            *app_state = local_app_state
        }
        state.state = Some(my_state);
        a
    }
}


/// Another implementation of the "use_state" pattern familiar in reactive UI.
///
/// In this version, the parent app data
/// is `T`, and the child is `S` where S is the local state.
///
/// The first callback creates the initial state (it is called on build but
/// not rebuild). The second callback updates both local and app state
/// after event processing. The third callback takes local state as an argument.
pub struct UseState2<T, A, S, V, FInit, FUpdate, F> {
    f_init: FInit,
    f_update: FUpdate,
    f: F,
    phantom: PhantomData<fn() -> (T, A, S, V)>,
}

pub struct UseStateState2<A, S, V: View<S, A>> {
    state: Option<S>,
    view: V,
    view_state: V::State,
}

impl<T, A, S, V, FInit: Fn() -> S, FUpdate: Fn(&mut S, &mut T), F: Fn(&mut S) -> V> UseState2<T, A, S, V, FInit, FUpdate, F> {
    #[allow(unused)]
    pub fn new(f_init: FInit, f_update: FUpdate, f: F) -> Self {
        let phantom = Default::default();
        UseState2 { f_init, f_update, f, phantom }
    }
}

impl<T, A, S, V: View<S, A>, FInit: Fn() -> S, FUpdate: Fn(&mut S, &mut T), F: Fn(&mut S) -> V> View<T, A>
    for UseState2<T, A, S, V, FInit, FUpdate, F>
where
    S: Send,
    FInit: Send,
    FUpdate: Send,
    F: Send,
{
    type State = UseStateState2<A, S, V>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let mut state = (self.f_init)();
        let view = (self.f)(&mut state);
        let (id, view_state, element) = view.build(cx);
        let my_state = UseStateState2 {
            state: Some(state),
            view,
            view_state,
        };
        (id, my_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        let view = (self.f)(state.state.as_mut().unwrap());
        let changed = view.rebuild(cx, &state.view, id, &mut state.view_state, element);
        state.view = view;
        changed
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        let mut local_state = state.state.take().unwrap();
        let a = state
            .view
            .event(id_path, &mut state.view_state, event, &mut local_state);

        (self.f_update)(&mut local_state, app_state);
        
        state.state = Some(local_state);
        a
    }
}
