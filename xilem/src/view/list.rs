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

//! A virtualized list of items.
//!
//! This is a hack for experimentation.

use std::{any::Any, collections::BTreeMap, marker::PhantomData};

use crate::{event::EventResult, id::Id, widget::Pod};

use super::{Cx, View};

pub struct List<T, A, V, F: Fn(usize) -> V> {
    n_items: usize,
    item_height: f64,
    callback: F,
    phantom: PhantomData<fn() -> (T, A)>,
}

pub struct ListState<T, A, V: View<T, A>> {
    add_req: Vec<usize>,
    remove_req: Vec<usize>,
    items: BTreeMap<usize, ItemState<T, A, V>>,
}

struct ItemState<T, A, V: View<T, A>> {
    id: Id,
    view: V,
    state: V::State,
}

pub fn list<T, A, V, F: Fn(usize) -> V>(
    n_items: usize,
    item_height: f64,
    callback: F,
) -> List<T, A, V, F> {
    List::new(n_items, item_height, callback)
}

impl<T, A, V, F: Fn(usize) -> V> List<T, A, V, F> {
    pub fn new(n_items: usize, item_height: f64, callback: F) -> Self {
        List {
            n_items,
            item_height,
            callback,
            phantom: Default::default(),
        }
    }
}

impl<T, A, V: View<T, A>, F: Fn(usize) -> V + Send> View<T, A> for List<T, A, V, F>
where
    V::Element: 'static,
{
    type State = ListState<T, A, V>;

    type Element = crate::widget::list::List;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| {
            crate::widget::list::List::new(cx.id_path().clone(), self.n_items, self.item_height)
        });
        let state = ListState {
            add_req: Vec::new(),
            remove_req: Vec::new(),
            items: BTreeMap::new(),
        };
        (id, state, element)
    }

    fn rebuild(
        &self,
        cx: &mut Cx,
        _prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        // TODO: allow updating of n_items and item_height
        let mut changed = !state.add_req.is_empty() || !state.remove_req.is_empty();
        cx.with_id(*id, |cx| {
            for (i, child_state) in &mut state.items {
                let child_view = (self.callback)(*i);
                let pod = element.child_mut(*i);
                let child_element = pod.downcast_mut().unwrap();
                let child_changed = child_view.rebuild(
                    cx,
                    &child_state.view,
                    &mut child_state.id,
                    &mut child_state.state,
                    child_element,
                );
                if child_changed {
                    pod.request_update();
                }
                changed |= child_changed;
                child_state.view = child_view;
            }
            for i in state.add_req.drain(..) {
                let child_view = (self.callback)(i);
                let (child_id, child_state, child_element) = child_view.build(cx);
                element.set_child(i, Pod::new(child_element));
                state.items.insert(
                    i,
                    ItemState {
                        id: child_id,
                        view: child_view,
                        state: child_state,
                    },
                );
            }
            for i in state.remove_req.drain(..) {
                element.remove_child(i);
                state.items.remove(&i);
            }
        });
        changed
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A> {
        if let Some((id, tl)) = id_path.split_first() {
            if let Some((_, s)) = state.items.iter_mut().find(|(_, s)| s.id == *id) {
                s.view.event(tl, &mut s.state, event, app_state)
            } else {
                EventResult::Stale
            }
        } else {
            let req: &crate::widget::list::ListChildRequest = event.downcast_ref().unwrap();
            state.add_req.extend(&req.add);
            state.remove_req.extend(&req.remove);
            EventResult::RequestRebuild
        }
    }
}
