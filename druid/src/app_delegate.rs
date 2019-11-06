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

use crate::{Command, Data, Env, Event, WinCtx, WindowId};

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
/// The `AppDelegate` is a struct that is allowed to handle and modify
/// events before they are passed down the widget tree.
///
/// It is a natural place for things like window and menu management.
///
/// You customize the `AppDelegate` by passing closures during creation.
pub struct AppDelegate<T> {
    event_fn: Option<
        Box<
            dyn Fn(Event, &mut T, &Env, &mut DelegateCtx, &mut dyn WinCtx) -> Option<Event>
                + 'static,
        >,
    >,
    window_added_fn: Option<Box<dyn Fn(WindowId, &mut T, &Env, &mut DelegateCtx)>>,
    window_removed_fn: Option<Box<dyn Fn(WindowId, &mut T, &Env, &mut DelegateCtx)>>,
}

impl<T: Data> AppDelegate<T> {
    /// Create a new `AppDelegate`.
    pub fn new() -> Self {
        AppDelegate {
            event_fn: None,
            window_added_fn: None,
            window_removed_fn: None,
        }
    }

    /// Set the `AppDelegate`'s event handler. This function receives all events,
    /// before they are passed down the tree.
    ///
    /// The return value of this function will be passed down the tree. This can
    /// be the even that was passed in, a different event, or no event. In all cases,
    /// the `update` method will be called as usual.
    pub fn event_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(Event, &mut T, &Env, &mut DelegateCtx, &mut dyn WinCtx) -> Option<Event> + 'static,
    {
        self.event_fn = Some(Box::new(f));
        self
    }

    pub fn window_added_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(WindowId, &mut T, &Env, &mut DelegateCtx) + 'static,
    {
        self.window_added_fn = Some(Box::new(f));
        self
    }

    pub fn window_removed_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(WindowId, &mut T, &Env, &mut DelegateCtx) + 'static,
    {
        self.window_removed_fn = Some(Box::new(f));
        self
    }

    pub(crate) fn event(
        &mut self,
        event: Event,
        data: &mut T,
        env: &Env,
        delegate_ctx: &mut DelegateCtx,
        win_ctx: &mut dyn WinCtx,
    ) -> Option<Event> {
        match self.event_fn.as_ref() {
            Some(f) => (f)(event, data, env, delegate_ctx, win_ctx),
            None => Some(event),
        }
    }

    pub(crate) fn window_added(
        &mut self,
        window_id: WindowId,
        data: &mut T,
        env: &Env,
        ctx: &mut DelegateCtx,
    ) {
        if let Some(f) = self.window_added_fn.as_ref() {
            f(window_id, data, env, ctx);
        }
    }

    pub(crate) fn window_removed(
        &mut self,
        window_id: WindowId,
        data: &mut T,
        env: &Env,
        ctx: &mut DelegateCtx,
    ) {
        if let Some(f) = self.window_removed_fn.as_ref() {
            f(window_id, data, env, ctx);
        }
    }
}
