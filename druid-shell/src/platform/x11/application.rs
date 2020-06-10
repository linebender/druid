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

//! X11 implementation of features at the application scope.

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::rc::Rc;

use anyhow::{anyhow, Context, Error};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, EventMask, WindowClass};
use x11rb::protocol::Event;
use x11rb::xcb_ffi::XCBConnection;

use crate::application::AppHandler;

use super::clipboard::Clipboard;
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
        let (conn, screen_num) = XCBConnection::connect(None)?;
        let connection = Rc::new(conn);
        let window_id = Application::create_event_window(&connection, screen_num as i32)?;
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            windows: HashMap::new(),
        }));
        Ok(Application {
            connection,
            screen_num: screen_num as i32,
            window_id,
            state,
        })
    }

    fn create_event_window(conn: &Rc<XCBConnection>, screen_num: i32) -> Result<u32, Error> {
        let id = conn.generate_id()?;
        let setup = conn.setup();
        let screen = setup
            .roots
            .get(screen_num as usize)
            .ok_or_else(|| anyhow!("invalid screen num: {}", screen_num))?;

        // Create the actual window
        if let Some(err) = conn
            .create_window(
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
            .check()?
        {
            // TODO: https://github.com/psychon/x11rb/pull/469 will make error handling easier with
            // the next x11rb release.
            return Err(x11rb::errors::ReplyError::X11Error(err).into());
        }

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
            //       CIRCULATE_NOTIFY, CONFIGURE_NOTIFY, GRAVITY_NOTIFY
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
            _ => {}
        }
        Ok(false)
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        loop {
            if let Ok(ev) = self.connection.wait_for_event() {
                match self.handle_event(&ev) {
                    Ok(quit) => {
                        if quit {
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Error handling event: {}", e);
                    }
                }
            }
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
                    log_x11!(self.connection.flush());
                }
            }
        } else {
            log::error!("Application state already borrowed");
        }
    }

    fn finalize_quit(&self) {
        log_x11!(self.connection.destroy_window(self.window_id));
        log_x11!(self.connection.flush());
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
}
