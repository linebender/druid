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

use druid_shell::kurbo::Size;

use crate::{event::EventResult, id::Id};

use super::{Cx, View};

pub struct LayoutObserver<T, A, F, V> {
    callback: F,
    phantom: PhantomData<(T, A, V)>,
}

pub struct LayoutObserverState<T, A, V: View<T, A>> {
    size: Option<Size>,
    child_id: Option<Id>,
    child_view: Option<V>,
    child_state: Option<V::State>,
}

impl<T, A, F, V> LayoutObserver<T, A, F, V> {
    pub fn new(callback: F) -> Self {
        LayoutObserver {
            callback,
            phantom: Default::default(),
        }
    }
}

impl<T, A, F: Fn(Size) -> V, V: View<T, A>> View<T, A> for LayoutObserver<T, A, F, V>
where
    V::Element: 'static,
{
    type State = LayoutObserverState<T, A, V>;

    type Element = crate::widget::layout_observer::LayoutObserver;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) =
            cx.with_new_id(|cx| crate::widget::layout_observer::LayoutObserver::new(cx.id_path()));
        let child_state = LayoutObserverState {
            size: None,
            child_id: None,
            child_view: None,
            child_state: None,
        };
        (id, child_state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut crate::id::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        if let Some(size) = &state.size {
            let view = (self.callback)(*size);
            cx.with_id(*id, |cx| {
                if let (Some(id), Some(prev_view), Some(child_state)) = (
                    &mut state.child_id,
                    &state.child_view,
                    &mut state.child_state,
                ) {
                    let child_pod = element.child_mut().as_mut().unwrap();
                    let child_element = child_pod.downcast_mut().unwrap();
                    let changed = view.rebuild(cx, prev_view, id, child_state, child_element);
                    state.child_view = Some(view);
                    if changed {
                        child_pod.request_update();
                    }
                    changed
                } else {
                    let (child_id, child_state, child_element) = view.build(cx);
                    element.set_child(Box::new(child_element));
                    state.child_id = Some(child_id);
                    state.child_state = Some(child_state);
                    state.child_view = Some(view);
                    true
                }
            })
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
    ) -> EventResult<A> {
        if id_path.is_empty() {
            if let Ok(size) = event.downcast() {
                state.size = Some(*size);
            }
            EventResult::RequestRebuild
        } else {
            let tl = &id_path[1..];
            if let (Some(child_view), Some(child_state)) =
                (&state.child_view, &mut state.child_state)
            {
                child_view.event(tl, child_state, event, app_state)
            } else {
                EventResult::Stale
            }
        }
    }
}
