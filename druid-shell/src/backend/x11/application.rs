// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! X11 implementation of features at the application scope.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Error};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::present::ConnectionExt as _;
use x11rb::protocol::render::{self, ConnectionExt as _, Pictformat};
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xproto::{
    self, ConnectionExt, CreateWindowAux, EventMask, Timestamp, Visualtype, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::resource_manager::{
    new_from_default as new_resource_db_from_default, Database as ResourceDb,
};
use x11rb::xcb_ffi::XCBConnection;

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::util;
use super::window::Window;
use crate::backend::shared::linux;
use crate::backend::shared::xkb;

// This creates a `struct WindowAtoms` containing the specified atoms as members (along with some
// convenience methods to intern and query those atoms). We use the following atoms:
//
// WM_PROTOCOLS
//
// List of atoms that identify the communications protocols between
// the client and window manager in which the client is willing to participate.
//
// https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#wm_protocols_property
//
// WM_DELETE_WINDOW
//
// Including this atom in the WM_PROTOCOLS property on each window makes sure that
// if the window manager respects WM_DELETE_WINDOW it will send us the event.
//
// The WM_DELETE_WINDOW event is sent when there is a request to close the window.
// Registering for but ignoring this event means that the window will remain open.
//
// https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#window_deletion
//
// _NET_WM_PID
//
// A property containing the PID of the process that created the window.
//
// https://specifications.freedesktop.org/wm-spec/wm-spec-1.3.html#idm45805407915360
//
// _NET_WM_NAME
//
// A version of WM_NAME supporting UTF8 text.
//
// https://specifications.freedesktop.org/wm-spec/wm-spec-1.3.html#idm45805407982336
//
// UTF8_STRING
//
// The type of _NET_WM_NAME
//
// CLIPBOARD
//
// The name of the clipboard selection; used for implementing copy&paste
//
// PRIMARY
//
// The name of the primary selection; used for implementing "paste the currently selected text"
//
// TARGETS
//
// A target for getting the selection contents that answers with a list of supported targets
//
// INCR
//
// Type used for incremental selection transfers
x11rb::atom_manager! {
    pub(crate) AppAtoms: AppAtomsCookie {
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
        _NET_WM_PID,
        _NET_WM_NAME,
        UTF8_STRING,
        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_NORMAL,
        _NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
        _NET_WM_WINDOW_TYPE_TOOLTIP,
        _NET_WM_WINDOW_TYPE_DIALOG,
        CLIPBOARD,
        PRIMARY,
        TARGETS,
        INCR,
    }
}

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
    /// subtleties; see [x11rb event loop integration notes][1] for more details.
    /// Let's just avoid the issue altogether. As far as public API is concerned, this causes
    /// `druid_shell::WindowHandle` to be `!Send` and `!Sync`.
    ///
    /// [1]: https://github.com/psychon/x11rb/blob/41ab6610f44f5041e112569684fc58cd6d690e57/src/event_loop_integration.rs.
    marker: std::marker::PhantomData<*mut XCBConnection>,

    /// The type of visual used by the root window
    root_visual_type: Visualtype,
    /// The visual for windows with transparent backgrounds, if supported
    argb_visual_type: Option<Visualtype>,
    /// Pending events that need to be handled later
    pending_events: Rc<RefCell<VecDeque<Event>>>,
    /// The atoms that we need
    atoms: Rc<AppAtoms>,

    /// The X11 resource database used to query dpi.
    pub(crate) rdb: Rc<ResourceDb>,
    pub(crate) cursors: Cursors,
    /// The clipboard implementation
    clipboard: Clipboard,
    /// The clipboard implementation for the primary selection
    primary: Clipboard,
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
    screen_num: usize, // Needs a container when no longer const
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
    /// Support for the render extension in at least version 0.5?
    render_argb32_pictformat_cursor: Option<Pictformat>,
    /// Newest timestamp that we received
    timestamp: Rc<Cell<Timestamp>>,
}

/// The mutable `Application` state.
struct State {
    /// Whether `Application::quit` has already been called.
    quitting: bool,
    /// A collection of all the `Application` windows.
    windows: HashMap<u32, Rc<Window>>,
    xkb_state: xkb::State,
}

#[derive(Clone, Debug)]
pub(crate) struct Cursors {
    pub default: Option<xproto::Cursor>,
    pub text: Option<xproto::Cursor>,
    pub pointer: Option<xproto::Cursor>,
    pub crosshair: Option<xproto::Cursor>,
    pub not_allowed: Option<xproto::Cursor>,
    pub row_resize: Option<xproto::Cursor>,
    pub col_resize: Option<xproto::Cursor>,
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
        let rdb = Rc::new(new_resource_db_from_default(&conn)?);
        let xkb_context = xkb::Context::new();
        xkb_context.set_log_level(tracing::Level::DEBUG);
        use x11rb::protocol::xkb::ConnectionExt;
        conn.xkb_use_extension(1, 0)?
            .reply()
            .context("init xkb extension")?;
        let device_id = xkb_context
            .core_keyboard_device_id(&conn)
            .context("get core keyboard device id")?;

        let keymap = xkb_context
            .keymap_from_device(&conn, device_id)
            .context("key map from device")?;

        let xkb_state = keymap.state();
        let connection = Rc::new(conn);
        let window_id = Application::create_event_window(&connection, screen_num)?;
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            windows: HashMap::new(),
            xkb_state,
        }));

        let (idle_read, idle_write) = nix::unistd::pipe2(nix::fcntl::OFlag::O_NONBLOCK)?;

        let present_opcode = if std::env::var_os("DRUID_SHELL_DISABLE_X11_PRESENT").is_some() {
            // Allow disabling Present with an environment variable.
            None
        } else {
            match Application::query_present_opcode(&connection) {
                Ok(p) => p,
                Err(e) => {
                    tracing::info!("failed to find Present extension: {}", e);
                    None
                }
            }
        };

        let pictformats = connection.render_query_pict_formats()?;
        let render_create_cursor_supported = matches!(connection
            .extension_information(render::X11_EXTENSION_NAME)?
            .and_then(|_| connection.render_query_version(0, 5).ok())
            .map(|cookie| cookie.reply())
            .transpose()?,
            Some(version) if version.major_version >= 1 || version.minor_version >= 5);
        let render_argb32_pictformat_cursor = if render_create_cursor_supported {
            pictformats
                .reply()?
                .formats
                .iter()
                .find(|format| {
                    format.type_ == render::PictType::DIRECT
                        && format.depth == 32
                        && format.direct.red_shift == 16
                        && format.direct.red_mask == 0xff
                        && format.direct.green_shift == 8
                        && format.direct.green_mask == 0xff
                        && format.direct.blue_shift == 0
                        && format.direct.blue_mask == 0xff
                        && format.direct.alpha_shift == 24
                        && format.direct.alpha_mask == 0xff
                })
                .map(|format| format.id)
        } else {
            drop(pictformats);
            None
        };

        let handle = x11rb::cursor::Handle::new(connection.as_ref(), screen_num, &rdb)?.reply()?;
        let load_cursor = |cursor| {
            handle
                .load_cursor(connection.as_ref(), cursor)
                .map_err(|e| tracing::warn!("Unable to load cursor {}, error: {}", cursor, e))
                .ok()
        };

        let cursors = Cursors {
            default: load_cursor("default"),
            text: load_cursor("text"),
            pointer: load_cursor("pointer"),
            crosshair: load_cursor("crosshair"),
            not_allowed: load_cursor("not-allowed"),
            row_resize: load_cursor("row-resize"),
            col_resize: load_cursor("col-resize"),
        };

        let atoms = Rc::new(
            AppAtoms::new(&*connection)?
                .reply()
                .context("get X11 atoms")?,
        );

        let screen = connection
            .setup()
            .roots
            .get(screen_num)
            .ok_or_else(|| anyhow!("Invalid screen num: {}", screen_num))?;
        let root_visual_type = util::get_visual_from_screen(screen)
            .ok_or_else(|| anyhow!("Couldn't get visual from screen"))?;
        let argb_visual_type = util::get_argb_visual_type(&connection, screen)?;

        let timestamp = Rc::new(Cell::new(x11rb::CURRENT_TIME));
        let pending_events = Default::default();
        let clipboard = Clipboard::new(
            Rc::clone(&connection),
            screen_num,
            Rc::clone(&atoms),
            atoms.CLIPBOARD,
            Rc::clone(&pending_events),
            Rc::clone(&timestamp),
        );
        let primary = Clipboard::new(
            Rc::clone(&connection),
            screen_num,
            Rc::clone(&atoms),
            atoms.PRIMARY,
            Rc::clone(&pending_events),
            Rc::clone(&timestamp),
        );

        Ok(Application {
            connection,
            rdb,
            screen_num,
            window_id,
            state,
            idle_read,
            cursors,
            clipboard,
            primary,
            idle_write,
            present_opcode,
            root_visual_type,
            argb_visual_type,
            atoms,
            pending_events: Default::default(),
            marker: std::marker::PhantomData,
            render_argb32_pictformat_cursor,
            timestamp,
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
        tracing::info!(
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
        tracing::info!(
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

    /// Return the ARGB32 pictformat of the server, but only if RENDER's CreateCursor is supported
    #[inline]
    pub(crate) fn render_argb32_pictformat_cursor(&self) -> Option<Pictformat> {
        self.render_argb32_pictformat_cursor
    }

    fn create_event_window(conn: &Rc<XCBConnection>, screen_num: usize) -> Result<u32, Error> {
        let id = conn.generate_id()?;
        let setup = conn.setup();
        let screen = setup
            .roots
            .get(screen_num)
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
            WindowClass::INPUT_ONLY,
            // Visual ID
            x11rb::COPY_FROM_PARENT,
            // Window properties mask
            &CreateWindowAux::new().event_mask(EventMask::STRUCTURE_NOTIFY),
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
    pub(crate) fn screen_num(&self) -> usize {
        self.screen_num
    }

    #[inline]
    pub(crate) fn argb_visual_type(&self) -> Option<Visualtype> {
        // Check if a composite manager is running
        let atom_name = format!("_NET_WM_CM_S{}", self.screen_num);
        let owner = self
            .connection
            .intern_atom(false, atom_name.as_bytes())
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.atom)
            .and_then(|atom| self.connection.get_selection_owner(atom).ok())
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.owner);

        if Some(x11rb::NONE) == owner {
            tracing::debug!("_NET_WM_CM_Sn selection is unowned, not providing ARGB visual");
            None
        } else {
            self.argb_visual_type
        }
    }

    #[inline]
    pub(crate) fn root_visual_type(&self) -> Visualtype {
        self.root_visual_type
    }

    #[inline]
    pub(crate) fn atoms(&self) -> &AppAtoms {
        &self.atoms
    }

    /// Returns `Ok(true)` if we want to exit the main loop.
    fn handle_event(&self, ev: &Event) -> Result<bool, Error> {
        if ev.server_generated() {
            // Update our latest timestamp
            let timestamp = match ev {
                Event::KeyPress(ev) => ev.time,
                Event::KeyRelease(ev) => ev.time,
                Event::ButtonPress(ev) => ev.time,
                Event::ButtonRelease(ev) => ev.time,
                Event::MotionNotify(ev) => ev.time,
                Event::EnterNotify(ev) => ev.time,
                Event::LeaveNotify(ev) => ev.time,
                Event::PropertyNotify(ev) => ev.time,
                _ => self.timestamp.get(),
            };
            self.timestamp.set(timestamp);
        }
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
                let hw_keycode = ev.detail;
                let mut state = borrow_mut!(self.state)?;
                let key_event = state.xkb_state.key_event(
                    hw_keycode as _,
                    keyboard_types::KeyState::Down,
                    false,
                );

                w.handle_key_event(key_event);
            }
            Event::KeyRelease(ev) => {
                let w = self
                    .window(ev.event)
                    .context("KEY_PRESS - failed to get window")?;
                let hw_keycode = ev.detail;
                let mut state = borrow_mut!(self.state)?;
                let key_event =
                    state
                        .xkb_state
                        .key_event(hw_keycode as _, keyboard_types::KeyState::Up, false);

                w.handle_key_event(key_event);
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
                    w.handle_button_press(ev)?;
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
                    w.handle_button_release(ev)?;
                }
            }
            Event::MotionNotify(ev) => {
                let w = self
                    .window(ev.event)
                    .context("MOTION_NOTIFY - failed to get window")?;
                w.handle_motion_notify(ev)?;
            }
            Event::ClientMessage(ev) => {
                let w = self
                    .window(ev.window)
                    .context("CLIENT_MESSAGE - failed to get window")?;
                w.handle_client_message(ev);
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
                w.handle_destroy_notify(ev);

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
            Event::SelectionClear(ev) => {
                self.clipboard
                    .handle_clear(*ev)
                    .context("SELECTION_CLEAR event handling for clipboard")?;
                self.primary
                    .handle_clear(*ev)
                    .context("SELECTION_CLEAR event handling for primary")?;
            }
            Event::SelectionRequest(ev) => {
                self.clipboard
                    .handle_request(ev)
                    .context("SELECTION_REQUEST event handling for clipboard")?;
                self.primary
                    .handle_request(ev)
                    .context("SELECTION_REQUEST event handling for primary")?;
            }
            Event::PropertyNotify(ev) => {
                self.clipboard
                    .handle_property_notify(*ev)
                    .context("PROPERTY_NOTIFY event handling for clipboard")?;
                self.primary
                    .handle_property_notify(*ev)
                    .context("PROPERTY_NOTIFY event handling for primary")?;
            }
            Event::FocusIn(ev) => {
                let w = self
                    .window(ev.event)
                    .context("FOCUS_IN - failed to get window")?;
                w.handle_got_focus();
            }
            Event::FocusOut(ev) => {
                let w = self
                    .window(ev.event)
                    .context("FOCUS_OUT - failed to get window")?;
                w.handle_lost_focus();
            }
            Event::Error(e) => {
                // TODO: if an error is caused by the present extension, disable it and fall back
                // to copying pixels. This was blocked on
                // https://github.com/psychon/x11rb/issues/503 but no longer is
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
                state
                    .windows
                    .values()
                    .filter_map(|w| w.next_timeout())
                    .min()
            } else {
                tracing::error!("Getting next timeout, application state already borrowed");
                None
            };
            let next_idle_time = last_idle_time + timeout;

            self.connection.flush()?;

            // Deal with pending events
            let mut event = self.pending_events.borrow_mut().pop_front();

            // Before we poll on the connection's file descriptor, check whether there are any
            // events ready. It could be that XCB has some events in its internal buffers because
            // of something that happened during the idle loop.
            if event.is_none() {
                event = self.connection.poll_for_event()?;
            }

            if event.is_none() {
                poll_with_timeout(
                    &self.connection,
                    self.idle_read,
                    next_timeout,
                    next_idle_time,
                )
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
                        tracing::error!("Error handling event: {:#}", e);
                    }
                }
                event = self.connection.poll_for_event()?;
            }

            let now = Instant::now();
            if let Some(timeout) = next_timeout {
                if timeout <= now {
                    if let Ok(state) = self.state.try_borrow() {
                        let values = state.windows.values().cloned().collect::<Vec<_>>();
                        drop(state);
                        for w in values {
                            w.run_timers(now);
                        }
                    } else {
                        tracing::error!("In timer loop, application state already borrowed");
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
                    tracing::error!("In idle loop, application state already borrowed");
                }
            }
        }
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        if let Err(e) = self.run_inner() {
            tracing::error!("{}", e);
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
            tracing::error!("Application state already borrowed");
        }
    }

    fn finalize_quit(&self) {
        log_x11!(self.connection.destroy_window(self.window_id));
        if let Err(e) = nix::unistd::close(self.idle_read) {
            tracing::error!("Error closing idle_read: {}", e);
        }
        if let Err(e) = nix::unistd::close(self.idle_write) {
            tracing::error!("Error closing idle_write: {}", e);
        }
    }

    pub fn clipboard(&self) -> Clipboard {
        self.clipboard.clone()
    }

    pub fn get_locale() -> String {
        linux::env::locale()
    }

    pub(crate) fn idle_pipe(&self) -> RawFd {
        self.idle_write
    }
}

impl crate::platform::linux::ApplicationExt for crate::Application {
    fn primary_clipboard(&self) -> crate::Clipboard {
        self.backend_app.primary.clone().into()
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
            Err(nix::errno::Errno::EINTR) => {}
            // According to write(2), this is the outcome of reading an empty, O_NONBLOCK
            // pipe.
            Err(nix::errno::Errno::EAGAIN) => {
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
fn poll_with_timeout(
    conn: &Rc<XCBConnection>,
    idle: RawFd,
    timer_timeout: Option<Instant>,
    idle_timeout: Instant,
) -> Result<(), Error> {
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
        }

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
                    .unwrap_or(c_int::MAX - 1);
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

            Err(nix::errno::Errno::EINTR) => {
                now = Instant::now();
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
