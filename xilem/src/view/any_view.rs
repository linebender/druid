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

use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use crate::{event::EventResult, id::Id, widget::AnyWidget};

use super::{Cx, View};

/// A trait enabling type erasure of views.
///
/// The name is slightly misleading as it's not any view, but only ones
/// whose element is AnyWidget.
///
/// Making a trait which is generic over another trait bound appears to
/// be well beyond the capability of Rust's type system. If type-erased
/// views with other bounds are needed, the best approach is probably
/// duplication of the code, probably with a macro.
pub trait AnyView<T, A = ()> {
    fn as_any(&self) -> &dyn Any;

    fn dyn_build(&self, cx: &mut Cx) -> (Id, Box<dyn Any + Send>, Box<dyn AnyWidget>);

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, A>,
        id: &mut Id,
        state: &mut Box<dyn Any + Send>,
        element: &mut Box<dyn AnyWidget>,
    ) -> bool;

    fn dyn_event(
        &self,
        id_path: &[Id],
        state: &mut dyn Any,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A>;
}

impl<T, A, V: View<T, A> + 'static> AnyView<T, A> for V
where
    V::State: 'static,
    V::Element: AnyWidget + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_build(&self, cx: &mut Cx) -> (Id, Box<dyn Any + Send>, Box<dyn AnyWidget>) {
        let (id, state, element) = self.build(cx);
        (id, Box::new(state), Box::new(element))
    }

    fn dyn_rebuild(
        &self,
        cx: &mut Cx,
        prev: &dyn AnyView<T, A>,
        id: &mut Id,
        state: &mut Box<dyn Any + Send>,
        element: &mut Box<dyn AnyWidget>,
    ) -> bool {
        if let Some(prev) = prev.as_any().downcast_ref() {
            if let Some(state) = state.downcast_mut() {
                if let Some(element) = element.deref_mut().as_any_mut().downcast_mut() {
                    self.rebuild(cx, prev, id, state, element)
                } else {
                    println!("downcast of element failed in dyn_rebuild");
                    false
                }
            } else {
                println!("downcast of state failed in dyn_rebuild");
                false
            }
        } else {
            let (new_id, new_state, new_element) = self.build(cx);
            *id = new_id;
            *state = Box::new(new_state);
            *element = Box::new(new_element);
            true
        }
    }

    fn dyn_event(
        &self,
        id_path: &[Id],
        state: &mut dyn Any,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        if let Some(state) = state.downcast_mut() {
            self.event(id_path, state, event, app_state)
        } else {
            // Possibly softer failure?
            panic!("downcast error in dyn_event");
        }
    }
}

impl<T, A> View<T, A> for Box<dyn AnyView<T, A> + Send> {
    type State = Box<dyn Any + Send>;

    type Element = Box<dyn AnyWidget>;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        self.deref().dyn_build(cx)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        self.deref()
            .dyn_rebuild(cx, prev.deref(), id, state, element)
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        self.deref()
            .dyn_event(id_path, state.deref_mut(), event, app_state)
    }
}
