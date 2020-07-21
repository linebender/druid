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

//! X11 implementation of features at the application scope.

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Error};
use x11rb::connection::Connection;
use x11rb::protocol::present::ConnectionExt as _;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, EventMask, WindowClass};
use x11rb::protocol::Event;
use x11rb::xcb_ffi::XCBConnection;

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::util;
use super::window::Window;

#[derive(Clone)]
pub(crate) struct Application {
    /// The connection to the X server.
    ///
    /// This connection is associated with a single display.
    /// The X server might also host other displays.
    ///
    /// A display is a collection of screens.
    connection: Rc<XCBConnection>,
    /// An `XCBConnection` is *technically* safe to use from other threads, but there are
    /// subtleties; see https://github.com/psychon/x11rb/blob/41ab6610f44f5041e112569684fc58cd6d690e57/src/event_loop_integration.rs.
    /// Let's just avoid the issue altogether. As far as public API is concerned, this causes
    /// `druid_shell::WindowHandle` to be `!Send` and `!Sync`.
    marker: std::marker::PhantomData<*mut XCBConnection>,
    /// The default screen of the connected display.
    ///
    /// The connected display may also have additional screens.
    /// Moving windows between multiple screens is difficult and there is no support for it.
    /// The application would have to create a clone of its window on multiple screens
    /// and then fake the visual transfer.
    ///
    /// In practice multiple physical monitor drawing areas are present on a single screen.
    /// This is achieved via various X server extensions (XRandR/Xinerama/TwinView),
    /// with XRandR seeming like the best choice.
    screen_num: i32, // Needs a container when no longer const
    /// The X11 window id of this `Application`.
    ///
    /// This is an input-only non-visual X11 window that is created first during initialization,
    /// and it is destroyed last during `Application::finalize_quit`.
    /// This window is useful for receiving application level events without any real windows.
    ///
    /// This is constant for the lifetime of the `Application`.
    window_id: u32,
    /// The mutable `Application` state.
    state: Rc<RefCell<State>>,
    /// The read end of the "idle pipe", a pipe that allows the event loop to be woken up from
    /// other threads.
    idle_read: RawFd,
    /// The write end of the "idle pipe", a pipe that allows the event loop to be woken up from
    /// other threads.
    idle_write: RawFd,
    /// The major opcode of the Present extension, if it is supported.
    present_opcode: Option<u8>,
}

/// The mutable `Application` state.
struct State {
    /// Whether `Application::quit` has already been called.
    quitting: bool,
    /// A collection of all the `Application` windows.
    windows: HashMap<u32, Rc<Window>>,
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        // If we want to support OpenGL, we will need to open a connection with Xlib support (see
        // https://xcb.freedesktop.org/opengl/ for background).  There is some sample code for this
        // in the `rust-xcb` crate (see `connect_with_xlib_display`), although it may be missing
        // something: according to the link below, If you want to handle events through x11rb and
        // libxcb, you should call XSetEventQueueOwner(dpy, XCBOwnsEventQueue). Otherwise, libX11
        // might randomly eat your events / move them to its own event queue.
        //
        // https://github.com/linebender/druid/pull/1025#discussion_r442777892
        let (conn, screen_num) = XCBConnection::connect(None)?;
        let connection = Rc::new(conn);
        let window_id = Application::create_event_window(&connection, screen_num as i32)?;
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            windows: HashMap::new(),
        }));

        let (idle_read, idle_write) = nix::unistd::pipe2(nix::fcntl::OFlag::O_NONBLOCK)?;

        let present_opcode = if std::env::var_os("DRUID_SHELL_DISABLE_X11_PRESENT").is_some() {
            // Allow disabling Present with an environment variable.
            None
        } else {
            match Application::query_present_opcode(&connection) {
                Ok(p) => p,
                Err(e) => {
                    log::info!("failed to find Present extension: {}", e);
                    None
                }
            }
        };

        Ok(Application {
            connection,
            screen_num: screen_num as i32,
            window_id,
            state,
            idle_read,
            idle_write,
            present_opcode,
            marker: std::marker::PhantomData,
        })
    }

    // Check if the Present extension is supported, returning its opcode if it is.
    fn query_present_opcode(conn: &Rc<XCBConnection>) -> Result<Option<u8>, Error> {
        let query = conn
            .query_extension(b"Present")?
            .reply()
            .context("query Present extension")?;

        if !query.present {
            return Ok(None);
        }

        let opcode = Some(query.major_opcode);

        // If Present is there at all, version 1.0 should be supported. This code
        // shouldn't have a real effect; it's just a sanity check.
        let version = conn
            .present_query_version(1, 0)?
            .reply()
            .context("query Present version")?;
        log::info!(
            "X server supports Present version {}.{}",
            version.major_version,
            version.minor_version,
        );

        // We need the XFIXES extension to use regions. This code looks like it's just doing a
        // sanity check but it is *necessary*: XFIXES doesn't work until we've done version
        // negotiation
        // (https://www.x.org/releases/X11R7.7/doc/fixesproto/fixesproto.txt)
        let version = conn
            .xfixes_query_version(5, 0)?
            .reply()
            .context("query XFIXES version")?;
        log::info!(
            "X server supports XFIXES version {}.{}",
            version.major_version,
            version.minor_version,
        );

        Ok(opcode)
    }

    #[inline]
    pub(crate) fn present_opcode(&self) -> Option<u8> {
        self.present_opcode
    }

    fn create_event_window(conn: &Rc<XCBConnection>, screen_num: i32) -> Result<u32, Error> {
        let id = conn.generate_id()?;
        let setup = conn.setup();
        let screen = setup
            .roots
            .get(screen_num as usize)
            .ok_or_else(|| anyhow!("invalid screen num: {}", screen_num))?;

        // Create the actual window
        conn.create_window(
            // Window depth
            x11rb::COPY_FROM_PARENT.try_into().unwrap(),
            // The new window's ID
            id,
            // Parent window of this new window
            screen.root,
            // X-coordinate of the new window
            0,
            // Y-coordinate of the new window
            0,
            // Width of the new window
            1,
            // Height of the new window
            1,
            // Border width
            0,
            // Window class type
            WindowClass::InputOnly,
            // Visual ID
            x11rb::COPY_FROM_PARENT,
            // Window properties mask
            &CreateWindowAux::new().event_mask(EventMask::StructureNotify),
        )?
        .check()
        .context("create input-only window")?;

        Ok(id)
    }

    pub(crate) fn add_window(&self, id: u32, window: Rc<Window>) -> Result<(), Error> {
        borrow_mut!(self.state)?.windows.insert(id, window);
        Ok(())
    }

    /// Remove the specified window from the `Application` and return the number of windows left.
    fn remove_window(&self, id: u32) -> Result<usize, Error> {
        let mut state = borrow_mut!(self.state)?;
        state.windows.remove(&id);
        Ok(state.windows.len())
    }

    fn window(&self, id: u32) -> Result<Rc<Window>, Error> {
        borrow!(self.state)?
            .windows
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("No window with id {}", id))
    }

    #[inline]
    pub(crate) fn connection(&self) -> &Rc<XCBConnection> {
        &self.connection
    }

    #[inline]
    pub(crate) fn screen_num(&self) -> i32 {
        self.screen_num
    }

    /// Returns `Ok(true)` if we want to exit the main loop.
    fn handle_event(&self, ev: &Event) -> Result<bool, Error> {
        match ev {
            // NOTE: When adding handling for any of the following events,
            //       there must be a check against self.window_id
            //       to know if the event must be ignored.
            //       Otherwise there will be a "failed to get window" error.
            //
            //       CIRCULATE_NOTIFY, GRAVITY_NOTIFY
            //       MAP_NOTIFY, REPARENT_NOTIFY, UNMAP_NOTIFY
            Event::Expose(ev) => {
                let w = self
                    .window(ev.window)
                    .context("EXPOSE - failed to get window")?;
                w.handle_expose(ev).context("EXPOSE - failed to handle")?;
            }
            Event::KeyPress(ev) => {
                let w = self
                    .window(ev.event)
                    .context("KEY_PRESS - failed to get window")?;
                w.handle_key_press(ev)
                    .context("KEY_PRESS - failed to handle")?;
            }
            Event::ButtonPress(ev) => {
                let w = self
                    .window(ev.event)
                    .context("BUTTON_PRESS - failed to get window")?;

                // X doesn't have dedicated scroll events: it uses mouse buttons instead.
                // Buttons 4/5 are vertical; 6/7 are horizontal.
                if ev.detail >= 4 && ev.detail <= 7 {
                    w.handle_wheel(ev)
                        .context("BUTTON_PRESS - failed to handle wheel")?;
                } else {
                    w.handle_button_press(ev)
                        .context("BUTTON_PRESS - failed to handle")?;
                }
            }
            Event::ButtonRelease(ev) => {
                let w = self
                    .window(ev.event)
                    .context("BUTTON_RELEASE - failed to get window")?;
                if ev.detail >= 4 && ev.detail <= 7 {
                    // This is the release event corresponding to a mouse wheel.
                    // Ignore it: we already handled the press event.
                } else {
                    w.handle_button_release(ev)
                        .context("BUTTON_RELEASE - failed to handle")?;
                }
            }
            Event::MotionNotify(ev) => {
                let w = self
                    .window(ev.event)
                    .context("MOTION_NOTIFY - failed to get window")?;
                w.handle_motion_notify(ev)
                    .context("MOTION_NOTIFY - failed to handle")?;
            }
            Event::ClientMessage(ev) => {
                let w = self
                    .window(ev.window)
                    .context("CLIENT_MESSAGE - failed to get window")?;
                w.handle_client_message(ev)
                    .context("CLIENT_MESSAGE - failed to handle")?;
            }
            Event::DestroyNotify(ev) => {
                if ev.window == self.window_id {
                    // The destruction of the Application window means that
                    // we need to quit the run loop.
                    return Ok(true);
                }

                let w = self
                    .window(ev.window)
                    .context("DESTROY_NOTIFY - failed to get window")?;
                w.handle_destroy_notify(ev)
                    .context("DESTROY_NOTIFY - failed to handle")?;

                // Remove our reference to the Window and allow it to be dropped
                let windows_left = self
                    .remove_window(ev.window)
                    .context("DESTROY_NOTIFY - failed to remove window")?;
                // Check if we need to finalize a quit request
                if windows_left == 0 && borrow!(self.state)?.quitting {
                    self.finalize_quit();
                }
            }
            Event::ConfigureNotify(ev) => {
                if ev.window != self.window_id {
                    let w = self
                        .window(ev.window)
                        .context("CONFIGURE_NOTIFY - failed to get window")?;
                    w.handle_configure_notify(ev)
                        .context("CONFIGURE_NOTIFY - failed to handle")?;
                }
            }
            Event::PresentCompleteNotify(ev) => {
                let w = self
                    .window(ev.window)
                    .context("COMPLETE_NOTIFY - failed to get window")?;
                w.handle_complete_notify(ev)
                    .context("COMPLETE_NOTIFY - failed to handle")?;
            }
            Event::PresentIdleNotify(ev) => {
                let w = self
                    .window(ev.window)
                    .context("IDLE_NOTIFY - failed to get window")?;
                w.handle_idle_notify(ev)
                    .context("IDLE_NOTIFY - failed to handle")?;
            }
            Event::Error(e) => {
                // TODO: if an error is caused by the present extension, disable it and fall back
                // to copying pixels. This is blocked on
                // https://github.com/psychon/x11rb/issues/503
                return Err(x11rb::errors::ReplyError::from(e.clone()).into());
            }
            _ => {}
        }
        Ok(false)
    }

    fn run_inner(self) -> Result<(), Error> {
        // Try to figure out the refresh rate of the current screen. We run the idle loop at that
        // rate. The rate-limiting of the idle loop has two purposes:
        //  - When the present extension is disabled, we paint in the idle loop. By limiting the
        //    idle loop to the monitor's refresh rate, we aren't painting unnecessarily.
        //  - By running idle commands at a limited rate, we limit spurious wake-ups: if the X11
        //    connection is otherwise idle, we'll wake up at most once per frame, run *all* the
        //    pending idle commands, and then go back to sleep.
        let refresh_rate = util::refresh_rate(self.connection(), self.window_id).unwrap_or(60.0);
        let timeout = Duration::from_millis((1000.0 / refresh_rate) as u64);
        let mut last_idle_time = Instant::now();
        loop {
            // Figure out when the next wakeup needs to happen
            let next_timeout = if let Ok(state) = self.state.try_borrow() {
                state.windows
                    .values()
                    .filter_map(|w| w.next_timeout())
                    .min()
            } else {
                log::error!("Getting next timeout, application state already borrowed");
                None
            };
            let next_idle_time = last_idle_time + timeout;

            self.connection.flush()?;

            // Before we poll on the connection's file descriptor, check whether there are any
            // events ready. It could be that XCB has some events in its internal buffers because
            // of something that happened during the idle loop.
            let mut event = self.connection.poll_for_event()?;

            if event.is_none() {
                poll_with_timeout(&self.connection, self.idle_read, next_timeout, next_idle_time)
                    .context("Error while waiting for X11 connection")?;
            }

            while let Some(ev) = event {
                match self.handle_event(&ev) {
                    Ok(quit) => {
                        if quit {
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        log::error!("Error handling event: {:#}", e);
                    }
                }
                event = self.connection.poll_for_event()?;
            }

            let now = Instant::now();
            if let Some(timeout) = next_timeout {
                if timeout <= now {
                    if let Ok(state) = self.state.try_borrow() {
                        for w in state.windows.values() {
                            w.run_timers(now);
                        }
                    } else {
                        log::error!("In timer loop, application state already borrowed");
                    }
                }
            }
            if now >= next_idle_time {
                last_idle_time = now;
                drain_idle_pipe(self.idle_read)?;

                if let Ok(state) = self.state.try_borrow() {
                    for w in state.windows.values() {
                        w.run_idle();
                    }
                } else {
                    log::error!("In idle loop, application state already borrowed");
                }
            }
        }
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        if let Err(e) = self.run_inner() {
            log::error!("{}", e);
        }
    }

    pub fn quit(&self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            if !state.quitting {
                state.quitting = true;
                if state.windows.is_empty() {
                    // There are no windows left, so we can immediately finalize the quit.
                    self.finalize_quit();
                } else {
                    // We need to queue up the destruction of all our windows.
                    // Failure to do so will lead to resource leaks.
                    for window in state.windows.values() {
                        window.destroy();
                    }
                }
            }
        } else {
            log::error!("Application state already borrowed");
        }
    }

    fn finalize_quit(&self) {
        log_x11!(self.connection.destroy_window(self.window_id));
        if let Err(e) = nix::unistd::close(self.idle_read) {
            log::error!("Error closing idle_read: {}", e);
        }
        if let Err(e) = nix::unistd::close(self.idle_write) {
            log::error!("Error closing idle_write: {}", e);
        }
    }

    pub fn clipboard(&self) -> Clipboard {
        // TODO(x11/clipboard): implement Application::clipboard
        log::warn!("Application::clipboard is currently unimplemented for X11 platforms.");
        Clipboard {}
    }

    pub fn get_locale() -> String {
        // TODO(x11/locales): implement Application::get_locale
        log::warn!("Application::get_locale is currently unimplemented for X11 platforms. (defaulting to en-US)");
        "en-US".into()
    }

    pub(crate) fn idle_pipe(&self) -> RawFd {
        self.idle_write
    }
}

/// Clears out our idle pipe; `idle_read` should be the reading end of a pipe that was opened with
/// O_NONBLOCK.
fn drain_idle_pipe(idle_read: RawFd) -> Result<(), Error> {
    // Each write to the idle pipe adds one byte; it's unlikely that there will be much in it, but
    // read it 16 bytes at a time just in case.
    let mut read_buf = [0u8; 16];
    loop {
        match nix::unistd::read(idle_read, &mut read_buf[..]) {
            Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {}
            // According to write(2), this is the outcome of reading an empty, O_NONBLOCK
            // pipe.
            Err(nix::Error::Sys(nix::errno::Errno::EAGAIN)) => {
                break;
            }
            Err(e) => {
                return Err(e).context("Failed to read from idle pipe");
            }
            // According to write(2), this is the outcome of reading an O_NONBLOCK pipe
            // when the other end has been closed. This shouldn't happen to us because we
            // own both ends, but just in case.
            Ok(0) => {
                break;
            }
            Ok(_) => {}
        }
    }
    Ok(())
}

/// Returns when there is an event ready to read from `conn`, or we got signalled by another thread
/// writing into our idle pipe and the `timeout` has passed.
// This was taken, with minor modifications, from the xclock_utc example in the x11rb crate.
// https://github.com/psychon/x11rb/blob/a6bd1453fd8e931394b9b1f2185fad48b7cca5fe/examples/xclock_utc.rs
fn poll_with_timeout(conn: &Rc<XCBConnection>, idle: RawFd, timer_timeout: Option<Instant>, idle_timeout: Instant) -> Result<(), Error> {
    use nix::poll::{poll, PollFd, PollFlags};
    use std::os::raw::c_int;
    use std::os::unix::io::AsRawFd;

    let mut now = Instant::now();
    let earliest_timeout = idle_timeout.min(timer_timeout.unwrap_or(idle_timeout));
    let fd = conn.as_raw_fd();
    let mut both_poll_fds = [
        PollFd::new(fd, PollFlags::POLLIN),
        PollFd::new(idle, PollFlags::POLLIN),
    ];
    let mut just_connection = [PollFd::new(fd, PollFlags::POLLIN)];
    let mut poll_fds = &mut both_poll_fds[..];

    // We start with no timeout in the poll call. If we get something from the idle handler, we'll
    // start setting one.
    let mut honor_idle_timeout = false;
    loop {
        fn readable(p: PollFd) -> bool {
            p.revents()
                .unwrap_or_else(PollFlags::empty)
                .contains(PollFlags::POLLIN)
        };

        // Compute the deadline for when poll() has to wakeup
        let deadline = if honor_idle_timeout {
            Some(earliest_timeout)
        } else {
            timer_timeout
        };
        // ...and convert the deadline into an argument for poll()
        let poll_timeout = if let Some(deadline) = deadline {
            if deadline <= now {
                break;
            } else {
                let millis = c_int::try_from(deadline.duration_since(now).as_millis())
                    .unwrap_or(c_int::max_value() - 1);
                // The above .as_millis() rounds down. This means we would wake up before the
                // deadline is reached. Add one to 'simulate' rounding up instead.
                millis + 1
            }
        } else {
            // No timeout
            -1
        };

        match poll(poll_fds, poll_timeout) {
            Ok(_) => {
                if readable(poll_fds[0]) {
                    // There is an X11 event ready to be handled.
                    break;
                }
                now = Instant::now();
                if timer_timeout.is_some() && now >= timer_timeout.unwrap() {
                    break;
                }
                if poll_fds.len() == 1 || readable(poll_fds[1]) {
                    // Now that we got signalled, stop polling from the idle pipe and use a timeout
                    // instead.
                    poll_fds = &mut just_connection;
                    honor_idle_timeout = true;
                    if now >= idle_timeout {
                        break;
                    }
                }
            }

            Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {
                now = Instant::now();
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
