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
use std::sync::mpsc::{channel, Receiver, Sender};

use static_assertions as sa;

use crate::shell::IdleHandle;
use crate::win_handler::EXT_EVENT_IDLE_TOKEN;
use crate::{command::SelectorSymbol, Command, Selector, Target};

pub(crate) type ExtCommand = (SelectorSymbol, Box<dyn Any + Send>, Option<Target>);
sa::assert_impl_all!(ExtCommand: Send);

/// A thing that can move into other threads and be used to submit commands back
/// to the running application.
///
/// This API is preliminary, and may be changed or removed without warning.
#[derive(Clone)]
pub struct ExtEventSink {
    tx: Sender<ExtCommand>,
    handle: Option<IdleHandle>,
}

sa::assert_impl_all!(ExtEventSink: Send);

/// The stuff that we hold onto inside the app that is related to the
/// handling of external events.
pub(crate) struct ExtEventHost {
    /// The items that have been sent to us.
    rx: Receiver<ExtCommand>,
    /// The sending end of `rx`. We don't use this ourselves, but we hand out
    /// a copy when creating an `ExtEventSink`.
    tx: Sender<ExtCommand>,
    /// This also needs to get passed to the event sinks, so that they can
    /// notify the idle loop that there are events to be processed.
    ///
    /// This is an `Option` because `druid-shell` doesn't guarantee that we'll get one.
    /// In practice, it seems like we only fail to get one if the window was closed.
    /// So in the absence of an idle handle, we just print warnings instead of sending
    /// commands.
    handle: Option<IdleHandle>,
}

/// An error that occurs if an external event cannot be submitted.
/// This probably means that the application has gone away.
#[derive(Debug, Clone)]
pub struct ExtEventError;

impl ExtEventHost {
    pub(crate) fn new(handle: Option<IdleHandle>) -> Self {
        let (tx, rx) = channel();
        ExtEventHost { rx, tx, handle }
    }

    pub(crate) fn make_sink(&self) -> ExtEventSink {
        ExtEventSink {
            tx: self.tx.clone(),
            handle: self.handle.clone(),
        }
    }

    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = (Option<Target>, Command)> + 'a {
        self.rx
            .try_iter()
            .map(|(sel, obj, targ)| (targ, Command::from_ext(sel, obj)))
    }
}

impl ExtEventSink {
    /// Submit a [`Command`] to the running application.
    ///
    /// [`Command`] is not thread safe, so you cannot submit it directly;
    /// instead you have to pass the [`Selector`] and the payload
    /// separately, and it will be turned into a `Command` when it is received.
    ///
    /// The `payload` must implement `Any + Send + Sync`.
    ///
    /// If no explicit `Target` is submitted, the `Command` will be sent to
    /// the window that created this event sink.
    ///
    /// [`Command`]: struct.Command.html
    /// [`Selector`]: struct.Selector.html
    pub fn submit_command<T: Any + Send + Sync>(
        &self,
        selector: Selector<T>,
        payload: impl Into<Box<T>>,
        target: impl Into<Option<Target>>,
    ) -> Result<(), ExtEventError> {
        let target = target.into();
        let payload = payload.into();
        if let Some(handle) = &self.handle {
            handle.schedule_idle(EXT_EVENT_IDLE_TOKEN);
            if self.tx.send((selector.symbol(), payload, target)).is_err() {
                Err(ExtEventError)
            } else {
                Ok(())
            }
        } else {
            Err(ExtEventError)
        }
    }
}

impl std::fmt::Display for ExtEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Window missing for external event")
    }
}

impl std::error::Error for ExtEventError {}
