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

use crate::{event::EventResult, id::Id};

use super::{Cx, View};

pub struct Adapt<T, A, U, B, F: Fn(&mut T, AdaptThunk<U, B, C>) -> EventResult<A>, C: View<U, B>> {
    f: F,
    child: C,
    phantom: PhantomData<fn() -> (T, A, U, B)>,
}

/// A "thunk" which dispatches an event to an adapt node's child.
///
/// The closure passed to Adapt should call this thunk with the child's
/// app state.
pub struct AdaptThunk<'a, U, B, C: View<U, B>> {
    child: &'a C,
    state: &'a mut C::State,
    id_path: &'a [Id],
    event: Box<dyn Any>,
}

impl<T, A, U, B, F: Fn(&mut T, AdaptThunk<U, B, C>) -> EventResult<A>, C: View<U, B>>
    Adapt<T, A, U, B, F, C>
{
    pub fn new(f: F, child: C) -> Self {
        Adapt {
            f,
            child,
            phantom: Default::default(),
        }
    }
}

impl<'a, U, B, C: View<U, B>> AdaptThunk<'a, U, B, C> {
    pub fn call(self, app_state: &mut U) -> EventResult<B> {
        self.child
            .event(self.id_path, self.state, self.event, app_state)
    }
}

impl<T, A, U, B, F: Fn(&mut T, AdaptThunk<U, B, C>) -> EventResult<A> + Send, C: View<U, B>>
    View<T, A> for Adapt<T, A, U, B, F, C>
{
    type State = C::State;

    type Element = C::Element;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        self.child.build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        self.child.rebuild(cx, &prev.child, id, state, element)
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        let thunk = AdaptThunk {
            child: &self.child,
            state,
            id_path,
            event,
        };
        (self.f)(app_state, thunk)
    }
}
