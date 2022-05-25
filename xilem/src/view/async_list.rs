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

//! A virtualized list of items that creates its view asynchronously.
//!
//! This is a hack for experimentation.

use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    future::Future,
    marker::PhantomData,
    pin::Pin,
};

use futures_task::{Context, Poll};
use smol::Task;

use crate::{event::EventResult, id::Id, widget::Pod};

use super::{Cx, View};

pub struct AsyncList<T, A, V, FF, F: Fn(usize) -> FF> {
    n_items: usize,
    item_height: f64,
    callback: F,
    phantom: PhantomData<(T, A, V)>,
}

pub struct AsyncListState<T, A, V: View<T, A>> {
    add_req: Vec<usize>,
    remove_req: Vec<usize>,
    items: BTreeMap<usize, ItemState<T, A, V>>,
    pending: HashMap<Id, (usize, Task<V>)>,
    waking: Vec<(Id, usize, Task<V>)>,
}

struct ItemState<T, A, V: View<T, A>> {
    id: Id,
    view: V,
    state: V::State,
}

pub fn async_list<T, A, V, FF, F: Fn(usize) -> FF>(
    n_items: usize,
    item_height: f64,
    callback: F,
) -> AsyncList<T, A, V, FF, F> {
    AsyncList::new(n_items, item_height, callback)
}

impl<T, A, V, FF, F: Fn(usize) -> FF> AsyncList<T, A, V, FF, F> {
    pub fn new(n_items: usize, item_height: f64, callback: F) -> Self {
        AsyncList {
            n_items,
            item_height,
            callback,
            phantom: Default::default(),
        }
    }
}

impl<T, A, V: View<T, A>, FF, F: Fn(usize) -> FF> View<T, A> for AsyncList<T, A, V, FF, F>
where
    FF: Future<Output = V> + Send + 'static,
    V: Send + 'static,
    V::Element: 'static,
{
    type State = AsyncListState<T, A, V>;

    type Element = crate::widget::list::List;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| {
            crate::widget::list::List::new(cx.id_path().clone(), self.n_items, self.item_height)
        });
        let state = AsyncListState {
            add_req: Vec::new(),
            remove_req: Vec::new(),
            items: BTreeMap::new(),
            pending: HashMap::new(),
            waking: Vec::new(),
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
        let mut changed = !state.waking.is_empty() || !state.remove_req.is_empty();
        cx.with_id(*id, |cx| {
            for i in state.add_req.drain(..) {
                // spawn a task to run the callback
                let future = (self.callback)(i);
                let join_handle = smol::spawn(Box::pin(future));
                let id = Id::next();
                state.waking.push((id, i, join_handle));
            }
            for (id, i, mut join_handle) in state.waking.drain(..) {
                let poll_result = cx.with_id(id, |cx| {
                    let waker = cx.waker();
                    let mut future_cx = Context::from_waker(&waker);
                    Pin::new(&mut join_handle).poll(&mut future_cx)
                });
                match poll_result {
                    Poll::Ready(v) => {
                        let child_view = v;
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
                        changed = true;
                    }
                    Poll::Pending => {
                        //println!("poll result id={:?} i={} pending, re-inserting", id, i);
                        state.pending.insert(id, (i, join_handle));
                    }
                }
            }
            for i in state.remove_req.drain(..) {
                element.remove_child(i);
                state.items.remove(&i);
            }
            // Note: we're not running rebuild on futures once resolved.
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
            if let Some((i, join_handle)) = state.pending.remove(id) {
                //println!("got async wake of id {:?}, i={}", id, i);
                state.waking.push((*id, i, join_handle));
                EventResult::RequestRebuild
            } else if let Some((_, s)) = state.items.iter_mut().find(|(_, s)| s.id == *id) {
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
