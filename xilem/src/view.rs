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
pub mod async_list;
pub mod button;
pub mod layout_observer;
pub mod list;
pub mod memoize;
pub mod scroll_view;
pub mod text;
pub mod use_state;
pub mod vstack;

use std::{any::Any, sync::Arc};

use druid_shell::{IdleHandle, IdleToken};
use futures_task::{ArcWake, Waker};

use crate::{
    app::WakeQueue,
    event::EventResult,
    id::{Id, IdPath},
    widget::Widget,
};

/// A view object representing a node in the UI.
///
/// This is a central trait for representing UI. An app will generate a tree of
/// these objects (the view tree) as the primary interface for expressing UI.
/// The view tree is transitory and is retained only long enough to dispatch
/// events and then serve as a reference for diffing for the next view tree.
///
/// The framework will then run methods on these views to create the associated
/// state tree and widget tree, as well as incremental updates and event
/// propagation.
///
/// The `View` trait is parameterized by `T`, which is known as the "app state",
/// and also a type for actions which are passed up the tree in event
/// propagation. During event handling, mutable access to the app state is
/// given to view nodes, which in turn can make expose it to callbacks.
pub trait View<T, A = ()>: Send {
    /// Associated state for the view.
    type State: Send;

    /// The associated widget for the view.
    type Element: Widget;

    /// Build the associated widget and initialize state.
    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element);

    /// Update the associated widget.
    ///
    /// Returns `true` when anything has changed.
    fn rebuild(
        &self,
        cx: &mut Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool;

    /// Propagate an event.
    ///
    /// Handle an event, propagating to children if needed. Here, `id_path` is a slice
    /// of ids beginning at a child of this view.
    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> EventResult<A>;
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
        //println!("path = {:?}", arc_self.id_path);
        if arc_self.wake_queue.push_wake(arc_self.id_path.clone()) {
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

    /// Run some logic with an id added to the id path.
    ///
    /// This is an ergonomic helper that ensures proper nesting of the id path.
    pub fn with_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, id: Id, f: F) -> T {
        self.push(id);
        let result = f(self);
        self.pop();
        result
    }

    /// Allocate a new id and run logic with the new id added to the id path.
    ///
    /// Also an ergonomic helper.
    pub fn with_new_id<T, F: FnOnce(&mut Cx) -> T>(&mut self, f: F) -> (Id, T) {
        let id = Id::next();
        self.push(id);
        let result = f(self);
        self.pop();
        (id, result)
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
