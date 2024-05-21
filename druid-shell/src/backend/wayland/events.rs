// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Multiplexing events.
//!
//! Calloop is a wrapper around `epoll` essentially allowing us to *select* multiple file
//! descriptors. We use it here to select events from a timer and from wayland.
//!
//! Based on `client-toolkit/src/event_loop.rs` in `smithay-client-toolkit` (MIT Licensed).

use calloop::{
    generic::{Fd, Generic},
    Dispatcher, EventSource, Interest, Mode,
};
use std::{cell::RefCell, io, rc::Rc};
use wayland_client::EventQueue;

use super::{application, window};

/// A wrapper around the wayland event queue that calloop knows how to select.
pub(crate) struct WaylandSource {
    appdata: std::sync::Arc<application::Data>,
    queue: Rc<RefCell<EventQueue>>,
    fd: Generic<Fd>,
}

impl WaylandSource {
    /// Wrap an `EventQueue` as a `WaylandSource`.
    pub fn new(appdata: std::sync::Arc<application::Data>) -> WaylandSource {
        let queue = appdata.wayland.queue.clone();
        let fd = queue.borrow().display().get_connection_fd();
        WaylandSource {
            appdata,
            queue,
            fd: Generic::from_fd(fd, Interest::READ, Mode::Level),
        }
    }

    /// Get a dispatcher that we can insert into our event loop.
    pub fn into_dispatcher(
        self,
    ) -> Dispatcher<
        Self,
        impl FnMut(
            window::WindowHandle,
            &mut Rc<RefCell<EventQueue>>,
            &mut std::sync::Arc<application::Data>,
        ) -> io::Result<u32>,
    > {
        Dispatcher::new(self, |_winhandle, queue, appdata| {
            queue
                .borrow_mut()
                .dispatch_pending(appdata, |event, object, _| {
                    tracing::error!(
                        "[druid-shell] Encountered an orphan event: {}@{} : {}",
                        event.interface,
                        object.as_ref().id(),
                        event.name
                    );
                    tracing::error!("all events should be handled: please raise an issue");
                })
        })
    }
}

impl EventSource for WaylandSource {
    type Event = window::WindowHandle;
    type Metadata = Rc<RefCell<EventQueue>>;
    type Ret = io::Result<u32>;

    fn process_events<F>(
        &mut self,
        ready: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<()>
    where
        F: FnMut(window::WindowHandle, &mut Rc<RefCell<EventQueue>>) -> Self::Ret,
    {
        tracing::trace!("processing events invoked {:?} {:?}", ready, token);

        self.appdata.display_flushed.replace(false);

        let winhandle = match self.appdata.acquire_current_window() {
            Some(winhandle) => winhandle,
            None => {
                tracing::error!("unable to acquire current window");
                return Ok(());
            }
        };

        // in case of readiness of the wayland socket we do the following in a loop, until nothing
        // more can be read:
        loop {
            // 1. read events from the socket if any are available
            if let Some(guard) = self.queue.borrow().prepare_read() {
                // might be None if some other thread read events before us, concurrently
                if let Err(e) = guard.read_events() {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e);
                    }
                }
            }
            tracing::trace!("processing events initiated");
            // 2. dispatch any pending event in the queue
            // propagate orphan events to the user
            let ret = callback(winhandle.clone(), &mut self.queue);
            tracing::trace!("processing events completed {:?}", ret);
            match ret {
                Ok(0) => {
                    // no events were dispatched even after reading the socket,
                    // nothing more to do, stop here
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    // in case of error, forward it and fast-exit
                    return Err(e);
                }
            }
        }

        tracing::trace!("dispatching completed, flushing");
        // 3. Once dispatching is finished, flush the responses to the compositor
        if let Err(e) = self.queue.borrow().display().flush() {
            if e.kind() != io::ErrorKind::WouldBlock {
                // in case of error, forward it and fast-exit
                return Err(e);
            }

            // WouldBlock error means the compositor could not process all our messages
            // quickly. Either it is slowed down or we are a spammer.
            // Should not really happen, if it does we do nothing and will flush again later.
            tracing::warn!("unable to flush display: {:?}", e);
        } else {
            self.appdata.display_flushed.replace(true);
        }

        tracing::trace!("event queue completed");
        Ok(())
    }

    fn register(&mut self, poll: &mut calloop::Poll, token: calloop::Token) -> std::io::Result<()> {
        self.fd.register(poll, token)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token: calloop::Token,
    ) -> std::io::Result<()> {
        self.fd.reregister(poll, token)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> std::io::Result<()> {
        self.fd.unregister(poll)
    }
}
