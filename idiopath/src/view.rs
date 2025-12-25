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

pub mod adapt;
pub mod any_view;
pub mod button;
pub mod column;
pub mod future;
pub mod memoize;
pub mod use_state;

use std::{any::Any, sync::Arc};

use druid_shell::{IdleHandle, IdleToken};
use futures_task::{ArcWake, Waker};

use crate::{
    app::WakeQueue,
    event::{AsyncWake, Event},
    id::{Id, IdPath},
    widget::Widget,
};

pub trait View<T, A> {
    type State;

    type Element: Widget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element);

    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    );

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> A;
}

#[derive(Clone)]
pub struct Cx {
    id_path: IdPath,
    idle_handle: Option<IdleHandle>,
    wake_queue: WakeQueue,
}

struct MyWaker {
    id_path: IdPath,
    idle_handle: IdleHandle,
    wake_queue: WakeQueue,
}

impl ArcWake for MyWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        println!("path = {:?}", arc_self.id_path);
        let event = Event::new(arc_self.id_path.clone(), AsyncWake);
        if arc_self.wake_queue.push_event(event) {
            // The clone shouldn't be needed; schedule_idle should be &self I think
            arc_self
                .idle_handle
                .clone()
                .schedule_idle(IdleToken::new(42));
        }
    }
}

impl Cx {
    pub fn new(wake_queue: &WakeQueue) -> Self {
        Cx {
            id_path: Vec::new(),
            idle_handle: None,
            wake_queue: wake_queue.clone(),
        }
    }

    pub fn push(&mut self, id: Id) {
        self.id_path.push(id);
    }

    pub fn pop(&mut self) {
        self.id_path.pop();
    }

    pub fn is_empty(&self) -> bool {
        self.id_path.is_empty()
    }

    pub fn id_path(&self) -> &IdPath {
        &self.id_path
    }

    pub(crate) fn set_handle(&mut self, idle_handle: Option<IdleHandle>) {
        self.idle_handle = idle_handle;
    }

    pub fn waker(&self) -> Waker {
        futures_task::waker(Arc::new(MyWaker {
            id_path: self.id_path.clone(),
            idle_handle: self.idle_handle.as_ref().unwrap().clone(),
            wake_queue: self.wake_queue.clone(),
        }))
    }
}
