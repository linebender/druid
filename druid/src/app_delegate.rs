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

use crate::{Command, Data, Env, Event, Target, WindowId};

/// A context passed in to [`AppDelegate`] functions.
///
/// [`AppDelegate`]: trait.AppDelegate.html
pub struct DelegateCtx<'a> {
    pub(crate) command_queue: &'a mut VecDeque<(Target, Command)>,
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
    pub fn submit_command(
        &mut self,
        command: impl Into<Command>,
        target: impl Into<Option<Target>>,
    ) {
        let command = command.into();
        let target = target.into().unwrap_or(Target::Global);
        self.command_queue.push_back((target, command))
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
    /// The `AppDelegate`'s event handler. This function receives all
    /// non-command events, before they are passed down the tree.
    ///
    /// The return value of this function will be passed down the tree. This can
    /// be the event that was passed in, a different event, or no event. In all cases,
    /// the [`update()`] method will be called as usual.
    ///
    /// [`update()`]: trait.Widget.html#tymethod.update
    fn event(
        &mut self,
        ctx: &mut DelegateCtx,
        window_id: WindowId,
        event: Event,
        data: &mut T,
        env: &Env,
    ) -> Option<Event> {
        Some(event)
    }

    /// The `AppDelegate`s [`Command`] handler.
    ///
    /// This function is called with each ([`Target`], [`Command`]) pair before
    /// they are sent down the tree.
    ///
    /// If your implementation returns `true`, the command will be sent down
    /// the widget tree. Otherwise it will not.
    ///
    /// To do anything fancier than this, you can submit arbitary commands
    /// via [`DelegateCtx::submit_command`].
    ///
    /// [`Target`]: enum.Target.html
    /// [`Command`]: struct.Command.html
    /// [`DelegateCtx::submit_command`]: struct.DelegateCtx.html#method.submit_command
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: &Target,
        cmd: &Command,
        data: &mut T,
        env: &Env,
    ) -> bool {
        true
    }

    /// The handler for window creation events.
    /// This function is called after a window has been added,
    /// allowing you to customize the window creation behavior of your app.
    fn window_added(&mut self, id: WindowId, data: &mut T, env: &Env, ctx: &mut DelegateCtx) {}

    /// The handler for window deletion events.
    /// This function is called after a window has been removed.
    fn window_removed(&mut self, id: WindowId, data: &mut T, env: &Env, ctx: &mut DelegateCtx) {}
}
