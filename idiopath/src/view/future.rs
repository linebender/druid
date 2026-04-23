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

use std::{future::Future, marker::PhantomData, pin::Pin};

use futures_task::{Context, Poll, Waker};

use crate::id::Id;

use super::{Cx, View};

pub struct FutureView<
    T,
    A,
    FutureCB: Fn() -> F,
    F: Future<Output = D> + Unpin,
    D,
    ViewCB: Fn(Option<&D>) -> V,
    V: View<T, A>,
> {
    future_cb: FutureCB,
    view_cb: ViewCB,
    phantom: PhantomData<(T, A)>,
}

pub struct FutureState<T, A, F: Future<Output = D> + Unpin, D, V: View<T, A>> {
    child_id: Id,
    waker: Waker,
    awakened: bool,
    future: PendingFuture<D, F>,
    view: V,
    view_state: V::State,
}

enum PendingFuture<D, F: Future<Output = D>> {
    Pending(F),
    Ready(D),
}

impl<D, F: Future<Output = D> + Unpin> PendingFuture<D, F> {
    // Return true if the state changed
    fn invoke(&mut self, waker: &Waker) -> bool {
        if let PendingFuture::Pending(f) = self {
            let mut future_cx = Context::from_waker(&waker);
            match Pin::new(f).poll(&mut future_cx) {
                Poll::Ready(d) => {
                    *self = PendingFuture::Ready(d);
                    true
                }
                Poll::Pending => false,
            }
        } else {
            false
        }
    }
}

// Note: the Default bound on A here is not quite right. A better approach I
// think would be to have an EventResult enum with A as one variant, and others
// for "id path not found" (ie async wake delivered to node that was deleted)
// and "async wake success", which would set dirty bits on Memoize etc.
impl<
        T,
        A: Default,
        FutureCB: Fn() -> F,
        F: Future<Output = D> + Unpin,
        D,
        ViewCB: Fn(Option<&D>) -> V,
        V: View<T, A>,
    > FutureView<T, A, FutureCB, F, D, ViewCB, V>
{
    pub fn new(future_cb: FutureCB, view_cb: ViewCB) -> Self {
        FutureView {
            future_cb,
            view_cb,
            phantom: Default::default(),
        }
    }
}

impl<
        T,
        A: Default,
        FutureCB: Fn() -> F,
        F: Future<Output = D> + Unpin,
        D,
        ViewCB: Fn(Option<&D>) -> V,
        V: View<T, A>,
    > View<T, A> for FutureView<T, A, FutureCB, F, D, ViewCB, V>
{
    type State = FutureState<T, A, F, D, V>;

    type Element = V::Element;

    fn build(&self, cx: &mut Cx) -> (crate::id::Id, Self::State, Self::Element) {
        let mut future = PendingFuture::Pending((self.future_cb)());
        let id = Id::next();
        cx.push(id);
        let waker = cx.waker();
        future.invoke(&waker);
        let data = match &future {
            PendingFuture::Ready(d) => Some(d),
            _ => None,
        };
        let view = (self.view_cb)(data);

        let (child_id, view_state, element) = view.build(cx);
        cx.pop();
        let state = FutureState {
            child_id,
            waker,
            awakened: false,
            future,
            view,
            view_state,
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
    ) {
        if state.awakened && state.future.invoke(&state.waker) {
            let data = match &state.future {
                PendingFuture::Ready(d) => Some(d),
                _ => None,
            };
            let view = (self.view_cb)(data);
            cx.push(*id);
            view.rebuild(
                cx,
                &state.view,
                &mut state.child_id,
                &mut state.view_state,
                element,
            );
            cx.pop();
            state.view = view;
        }
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn std::any::Any>,
        app_state: &mut T,
    ) -> A {
        if id_path.is_empty() {
            state.awakened = true;
            Default::default()
        } else {
            state
                .view
                .event(&id_path[1..], &mut state.view_state, event, app_state)
        }
    }
}
