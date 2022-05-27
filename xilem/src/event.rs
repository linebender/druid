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

use std::any::Any;

use crate::id::IdPath;

pub struct Event {
    pub id_path: IdPath,
    pub body: Box<dyn Any + Send>,
}

/// A result wrapper type for event handlers.
pub enum EventResult<A> {
    /// The event handler was invoked and returned an action.
    Action(A),
    /// The event handler received a change request that requests a rebuild.
    #[allow(unused)]
    RequestRebuild,
    /// The event handler discarded the event.
    #[allow(unused)]
    Nop,
    /// The event was addressed to an id path no longer in the tree.
    ///
    /// This is a normal outcome for async operation when the tree is changing
    /// dynamically, but otherwise indicates a logic error.
    Stale,
}

pub struct AsyncWake;

impl<A> EventResult<A> {
    #[allow(unused)]
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> EventResult<B> {
        match self {
            EventResult::Action(a) => EventResult::Action(f(a)),
            EventResult::RequestRebuild => EventResult::RequestRebuild,
            EventResult::Stale => EventResult::Stale,
            EventResult::Nop => EventResult::Nop,
        }
    }
}

impl Event {
    pub fn new(id_path: IdPath, event: impl Any + Send) -> Event {
        Event {
            id_path,
            body: Box::new(event),
        }
    }
}
