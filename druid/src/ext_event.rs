// Copyright 2020 The Druid Authors.
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

//! Simple handle for submitting external events.

use std::any::Any;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::shell::IdleHandle;
use crate::win_handler::EXT_EVENT_IDLE_TOKEN;
use crate::{command::SelectorSymbol, Command, Selector, Target, WindowId};

pub(crate) type ExtCommand = (SelectorSymbol, Box<dyn Any + Send>, Target);

/// A thing that can move into other threads and be used to submit commands back
/// to the running application.
///
/// This API is preliminary, and may be changed or removed without warning.
#[derive(Clone)]
pub struct ExtEventSink {
    queue: Arc<Mutex<VecDeque<ExtCommand>>>,
    handle: Arc<Mutex<Option<IdleHandle>>>,
}

/// The stuff that we hold onto inside the app that is related to the
/// handling of external events.
#[derive(Default)]
pub(crate) struct ExtEventHost {
    /// A shared queue of items that have been sent to us.
    queue: Arc<Mutex<VecDeque<ExtCommand>>>,
    /// This doesn't exist when the app starts and it can go away if a window closes, so we keep a
    /// reference here and can update it when needed. Note that this reference is shared with all
    /// `ExtEventSink`s, so that we can update them too.
    handle: Arc<Mutex<Option<IdleHandle>>>,
    /// The window that the handle belongs to, so we can keep track of when
    /// we need to get a new handle.
    pub(crate) handle_window_id: Option<WindowId>,
}

/// An error that occurs if an external event cannot be submitted.
/// This probably means that the application has gone away.
#[derive(Debug, Clone)]
pub struct ExtEventError;

impl ExtEventHost {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn make_sink(&self) -> ExtEventSink {
        ExtEventSink {
            queue: self.queue.clone(),
            handle: self.handle.clone(),
        }
    }

    pub(crate) fn set_idle(&mut self, handle: IdleHandle, window_id: WindowId) {
        self.handle.lock().unwrap().replace(handle);
        self.handle_window_id = Some(window_id);
    }

    pub(crate) fn has_pending_items(&self) -> bool {
        !self.queue.lock().unwrap().is_empty()
    }

    pub(crate) fn recv(&mut self) -> Option<Command> {
        self.queue
            .lock()
            .unwrap()
            .pop_front()
            .map(|(selector, payload, target)| Command::from_ext(selector, payload, target))
    }
}

impl ExtEventSink {
    /// Submit a [`Command`] to the running application.
    ///
    /// [`Command`] is not thread safe, so you cannot submit it directly;
    /// instead you have to pass the [`Selector`] and the payload
    /// separately, and it will be turned into a [`Command`] when it is received.
    ///
    /// The `payload` must implement `Any + Send`.
    ///
    /// If the [`Target::Auto`] is equivalent to [`Target::Global`].
    ///
    /// [`Command`]: struct.Command.html
    /// [`Selector`]: struct.Selector.html
    /// [`Target::Auto`]: enum.Target.html#variant.Auto
    /// [`Target::Global`]: enum.Target.html#variant.Global
    pub fn submit_command<T: Any + Send>(
        &self,
        selector: Selector<T>,
        payload: impl Into<Box<T>>,
        target: impl Into<Target>,
    ) -> Result<(), ExtEventError> {
        let target = target.into();
        let payload = payload.into();
        if let Some(handle) = self.handle.lock().unwrap().as_mut() {
            handle.schedule_idle(EXT_EVENT_IDLE_TOKEN);
        }
        self.queue.lock().map_err(|_| ExtEventError)?.push_back((
            selector.symbol(),
            payload,
            target,
        ));
        Ok(())
    }
}

impl std::fmt::Display for ExtEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Window missing for external event")
    }
}

impl std::error::Error for ExtEventError {}
