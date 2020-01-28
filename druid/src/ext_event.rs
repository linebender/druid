// Copyright 2020 The xi-editor Authors.
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

use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};

use crate::shell::IdleHandle;
use crate::win_handler::EXT_EVENT_IDLE_TOKEN;
use crate::{ExtCommand, Target, WindowId};

/// A thing that can move into other threads and be used to submit commands back
/// to the running application.
///
/// This API is preliminary, and may be changed or removed without warning.
#[derive(Clone)]
pub struct ExtEventSink {
    pub(crate) send: Sender<(Option<Target>, ExtCommand)>,
    pub(crate) handle: Arc<Mutex<Option<IdleHandle>>>,
}

/// The stuff that we hold onto inside the app that is related to the
/// handling of external events.
pub(crate) struct ExtEventHost {
    /// a channel for sending; we clone this and give it to friends that
    /// we want to be able to send messages to us.
    send: Sender<(Option<Target>, ExtCommand)>,
    /// The channel where we receive our friends messages
    recv: Receiver<(Option<Target>, ExtCommand)>,
    /// A handle to a thing in druid-shell that we can ask to poke us when
    /// our friend sends us a message.
    ///
    /// This doesn't exist when the app starts and it can go away if a window
    /// closes, so we keep a reference here and can update it when needed.
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
        let (send, recv) = crossbeam_channel::unbounded();
        ExtEventHost {
            send,
            recv,
            handle: Arc::default(),
            handle_window_id: None,
        }
    }

    pub(crate) fn make_sink(&self) -> ExtEventSink {
        ExtEventSink {
            send: self.send.clone(),
            handle: self.handle.clone(),
        }
    }

    pub(crate) fn set_idle(&mut self, handle: IdleHandle, window_id: WindowId) {
        self.handle.lock().unwrap().replace(handle);
        self.handle_window_id = Some(window_id);
    }

    pub(crate) fn has_pending_items(&self) -> bool {
        !self.recv.is_empty()
    }

    pub(crate) fn recv(&mut self) -> Option<(Option<Target>, ExtCommand)> {
        self.recv.try_recv().ok()
    }
}

impl ExtEventSink {
    /// Submit an [`ExtCommand`] to the running application.
    ///
    /// If no explicit `Target` is submitted, the `Command` will be sent to
    /// the application's first window; if that window is subsequently closed,
    /// then the command will be sent to *an arbitrary other window*.
    ///
    /// This limitation may be removed in the future.
    ///
    /// [`ExtCommand`]: struct.ExtCommand.html
    pub fn submit_command(
        &self,
        command: impl Into<ExtCommand>,
        target: impl Into<Option<Target>>,
    ) -> Result<(), ExtEventError> {
        let target = target.into();
        if let Some(handle) = self.handle.lock().unwrap().as_mut() {
            handle.schedule_idle(EXT_EVENT_IDLE_TOKEN);
        }
        self.send
            .send((target, command.into()))
            .map_err(|_| ExtEventError)
    }
}

impl std::fmt::Display for ExtEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Window missing for external event")
    }
}

impl std::error::Error for ExtEventError {}
