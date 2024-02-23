// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::common_util::IdleCallback;
use crate::window;

/// This represents different Idle Callback Mechanism
pub(super) enum Kind {
    Callback(Box<dyn IdleCallback>),
    Token(window::IdleToken),
}

impl std::fmt::Debug for Kind {
    fn fmt(&self, format: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Kind::Callback(_) => format.debug_struct("Idle(Callback)").finish(),
            Kind::Token(token) => format
                .debug_struct("Idle(Token)")
                .field("token", &token)
                .finish(),
        }
    }
}

#[derive(Clone)]
pub struct Handle {
    pub(super) queue: std::sync::Arc<std::sync::Mutex<Vec<Kind>>>,
}

impl Handle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the main UI thread, and
    /// won't be scheduled if the associated view has been dropped.
    ///
    /// Note: the name "idle" suggests that it will be scheduled with a lower
    /// priority than other UI events, but that's not necessarily the case.
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&mut dyn window::WinHandler) + Send + 'static,
    {
        tracing::trace!("add_idle_callback initiated");
        let mut queue = self.queue.lock().unwrap();
        queue.push(Kind::Callback(Box::new(callback)));
    }

    pub fn add_idle_token(&self, token: window::IdleToken) {
        tracing::trace!("add_idle_token initiated {:?}", token);
        let mut queue = self.queue.lock().unwrap();
        queue.push(Kind::Token(token));
    }
}

pub(crate) fn run(state: &Handle, winhandle: &mut dyn window::WinHandler) {
    let queue: Vec<_> = std::mem::take(&mut state.queue.lock().unwrap());
    for item in queue {
        match item {
            Kind::Callback(it) => it.call(winhandle),
            Kind::Token(it) => winhandle.idle(it),
        }
    }
}
