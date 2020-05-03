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

use xcb::{
    ButtonPressEvent, ButtonReleaseEvent, ClientMessageEvent, Connection, DestroyNotifyEvent,
    ExposeEvent, KeyPressEvent, MotionNotifyEvent, BUTTON_PRESS, BUTTON_RELEASE, CLIENT_MESSAGE,
    COPY_FROM_PARENT, CW_EVENT_MASK, DESTROY_NOTIFY, EVENT_MASK_STRUCTURE_NOTIFY, EXPOSE,
    KEY_PRESS, MOTION_NOTIFY, WINDOW_CLASS_INPUT_ONLY,
};

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::error::Error;
use super::window::Window;

#[derive(Clone)]
pub(crate) struct Application {
    /// The connection to the X server.
    ///
    /// This connection is associated with a single display.
    /// The X server might also host other displays.
    ///
    /// A display is a collection of screens.
    connection: Rc<Connection>,
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
        let (conn, screen_num) = match Connection::connect_with_xlib_display() {
            Ok(conn) => conn,
            Err(err) => return Err(Error::ConnectionError(err.to_string())),
        };
        let connection = Rc::new(conn);
        let window_id = Application::create_event_window(&connection, screen_num)?;
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            windows: HashMap::new(),
        }));
        Ok(Application {
            connection,
            screen_num,
            window_id,
            state,
        })
    }

    fn create_event_window(conn: &Rc<Connection>, screen_num: i32) -> Result<u32, Error> {
        let id = conn.generate_id();
        let setup = conn.get_setup();
        // TODO(x11/errors): Don't unwrap for screen?
        let screen = setup.roots().nth(screen_num as usize).unwrap();

        let cw_values = [(CW_EVENT_MASK, EVENT_MASK_STRUCTURE_NOTIFY)];

        // Create the actual window
        // TODO(x11/errors): check that this actually succeeds?
        xcb::create_window(
            // Connection to the X server
            conn,
            // Window depth
            COPY_FROM_PARENT.try_into().unwrap(),
            // The new window's ID
            id,
            // Parent window of this new window
            screen.root(),
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
            WINDOW_CLASS_INPUT_ONLY as u16,
            // Visual ID
            COPY_FROM_PARENT.try_into().unwrap(),
            // Window properties mask
            &cw_values,
        );

        Ok(id)
    }

    pub(crate) fn add_window(&self, id: u32, window: Rc<Window>) -> Result<(), Error> {
        match self.state.try_borrow_mut() {
            Ok(mut state) => {
                state.windows.insert(id, window);
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Application::add_window state: {}",
                err
            ))),
        }
    }

    /// Remove the specified window from the `Application` and return the number of windows left.
    fn remove_window(&self, id: u32) -> Result<usize, Error> {
        match self.state.try_borrow_mut() {
            Ok(mut state) => {
                state.windows.remove(&id);
                Ok(state.windows.len())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Application::remove_window state: {}",
                err
            ))),
        }
    }

    fn window(&self, id: u32) -> Result<Rc<Window>, Error> {
        match self.state.try_borrow() {
            Ok(state) => state
                .windows
                .get(&id)
                .cloned()
                .ok_or_else(|| Error::Generic(format!("No window with id {}", id))),
            Err(err) => Err(Error::BorrowError(format!(
                "Application::window state: {}",
                err
            ))),
        }
    }

    #[inline]
    pub(crate) fn connection(&self) -> &Rc<Connection> {
        &self.connection
    }

    #[inline]
    pub(crate) fn screen_num(&self) -> i32 {
        self.screen_num
    }

    // TODO(x11/events): handle mouse scroll events
    #[allow(clippy::cognitive_complexity)]
    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        loop {
            if let Some(ev) = self.connection.wait_for_event() {
                let ev_type = ev.response_type() & !0x80;
                // NOTE: When adding handling for any of the following events,
                //       there must be a check against self.window_id
                //       to know if the event must be ignored.
                //       Otherwise there will be a "failed to get window" error.
                //
                //       CIRCULATE_NOTIFY, CONFIGURE_NOTIFY, GRAVITY_NOTIFY
                //       MAP_NOTIFY, REPARENT_NOTIFY, UNMAP_NOTIFY
                match ev_type {
                    EXPOSE => {
                        let expose = unsafe { xcb::cast_event::<ExposeEvent>(&ev) };
                        let window_id = expose.window();
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_expose(expose) {
                                    log::error!("EXPOSE - failed to handle: {}", err);
                                }
                            }
                            Err(err) => log::error!("EXPOSE - failed to get window: {}", err),
                        }
                    }
                    KEY_PRESS => {
                        let key_press = unsafe { xcb::cast_event::<KeyPressEvent>(&ev) };
                        let window_id = key_press.event();
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_key_press(key_press) {
                                    log::error!("KEY_PRESS - failed to handle: {}", err);
                                }
                            }
                            Err(err) => log::error!("KEY_PRESS - failed to get window: {}", err),
                        }
                    }
                    BUTTON_PRESS => {
                        let button_press = unsafe { xcb::cast_event::<ButtonPressEvent>(&ev) };
                        let window_id = button_press.event();
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_button_press(button_press) {
                                    log::error!("BUTTON_PRESS - failed to handle: {}", err);
                                }
                            }
                            Err(err) => log::error!("BUTTON_PRESS - failed to get window: {}", err),
                        }
                    }
                    BUTTON_RELEASE => {
                        let button_release = unsafe { xcb::cast_event::<ButtonReleaseEvent>(&ev) };
                        let window_id = button_release.event();
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_button_release(button_release) {
                                    log::error!("BUTTON_RELEASE - failed to handle: {}", err);
                                }
                            }
                            Err(err) => {
                                log::error!("BUTTON_RELEASE - failed to get window: {}", err)
                            }
                        }
                    }
                    MOTION_NOTIFY => {
                        let motion_notify = unsafe { xcb::cast_event::<MotionNotifyEvent>(&ev) };
                        let window_id = motion_notify.event();
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_motion_notify(motion_notify) {
                                    log::error!("MOTION_NOTIFY - failed to handle: {}", err);
                                }
                            }
                            Err(err) => {
                                log::error!("MOTION_NOTIFY - failed to get window: {}", err)
                            }
                        }
                    }
                    CLIENT_MESSAGE => {
                        let client_message = unsafe { xcb::cast_event::<ClientMessageEvent>(&ev) };
                        let window_id = client_message.window();
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_client_message(client_message) {
                                    log::error!("CLIENT_MESSAGE - failed to handle: {}", err);
                                }
                            }
                            Err(err) => {
                                log::error!("CLIENT_MESSAGE - failed to get window: {}", err)
                            }
                        }
                    }
                    DESTROY_NOTIFY => {
                        let destroy_notify = unsafe { xcb::cast_event::<DestroyNotifyEvent>(&ev) };
                        let window_id = destroy_notify.window();
                        if window_id == self.window_id {
                            // The destruction of the Application window means that
                            // we need to quit the run loop.
                            break;
                        }
                        match self.window(window_id) {
                            Ok(w) => {
                                if let Err(err) = w.handle_destroy_notify(destroy_notify) {
                                    log::error!("DESTROY_NOTIFY - failed to handle: {}", err);
                                }
                            }
                            Err(err) => {
                                log::error!("DESTROY_NOTIFY - failed to get window: {}", err)
                            }
                        }

                        // Remove our reference to the Window and allow it to be dropped
                        match self.remove_window(window_id) {
                            Ok(windows_left) => {
                                if windows_left == 0 {
                                    // Check if we need to finalize a quit request
                                    if let Ok(state) = self.state.try_borrow() {
                                        if state.quitting {
                                            self.finalize_quit();
                                        }
                                    } else {
                                        log::error!(
                                            "DESTROY_NOTIFY - failed to check for quit request"
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                log::error!("DESTROY_NOTIFY - failed to remove window: {}", err)
                            }
                        }
                    }
                    _ => {}
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
                    self.connection.flush();
                }
            }
        } else {
            log::error!("Application state already borrowed");
        }
    }

    fn finalize_quit(&self) {
        xcb::destroy_window(&self.connection, self.window_id);
        self.connection.flush();
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
