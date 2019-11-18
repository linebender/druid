// Copyright 2019 The xi-editor Authors.
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

//! Customizing application-level behaviour.

use std::collections::VecDeque;

use crate::{Command, Data, Env, Event, WindowId};

/// A context passed in to [`AppDelegate`] functions.
pub struct DelegateCtx<'a> {
    pub(crate) source_id: WindowId,
    pub(crate) command_queue: &'a mut VecDeque<(WindowId, Command)>,
}

impl<'a> DelegateCtx<'a> {
    /// Submit a [`Command`] to be run after this event is handled.
    ///
    /// Commands are run in the order they are submitted; all commands
    /// submitted during the handling of an event are executed before
    /// the [`update()`] method is called.
    ///
    /// [`Command`]: struct.Command.html
    /// [`update()`]: trait.Widget.html#tymethod.update
    pub fn submit_command(&mut self, command: Command, window_id: impl Into<Option<WindowId>>) {
        let window_id = window_id.into().unwrap_or(self.source_id);
        self.command_queue.push_back((window_id, command))
    }
}

/// A type that provides hooks for handling and modifying top-level events.
///
/// The `AppDelegate` is a trait that is allowed to handle and modify
/// events before they are passed down the widget tree.
///
/// It is a natural place for things like window and menu management.
///
/// You customize the `AppDelegate` by implementing its methods on your own type.
#[allow(unused)]
pub trait AppDelegate<T: Data> {
    /// The `AppDelegate`'s event handler. This function receives all events,
    /// before they are passed down the tree.
    ///
    /// The return value of this function will be passed down the tree. This can
    /// be the even that was passed in, a different event, or no event. In all cases,
    /// the `update` method will be called as usual.
    fn event(
        &mut self,
        event: Event,
        data: &mut T,
        env: &Env,
        ctx: &mut DelegateCtx,
    ) -> Option<Event> {
        Some(event)
    }

    /// The handler for window creation events.
    /// This function is called after a window has been added,
    /// allowing you to customize the window creation behavior of your app.
    fn window_added(&mut self, id: WindowId, data: &mut T, env: &Env, ctx: &mut DelegateCtx) {}

    /// The handler for window deletion events.
    /// This function is called after a window has been removed.
    fn window_removed(&mut self, id: WindowId, data: &mut T, env: &Env, ctx: &mut DelegateCtx) {}
}
