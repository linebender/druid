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
use std::rc::Rc;

use xcb::{
    ButtonPressEvent, ButtonReleaseEvent, ClientMessageEvent, Connection, DestroyNotifyEvent,
    ExposeEvent, KeyPressEvent, MotionNotifyEvent, BUTTON_PRESS, BUTTON_RELEASE, CLIENT_MESSAGE,
    DESTROY_NOTIFY, EXPOSE, KEY_PRESS, MOTION_NOTIFY,
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
    /// The mutable `Application` state.
    state: Rc<RefCell<State>>,
}

/// The mutable `Application` state.
struct State {
    /// A collection of all the `Application` windows.
    windows: HashMap<u32, Rc<Window>>,
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        let (conn, screen_num) = match Connection::connect_with_xlib_display() {
            Ok(conn) => conn,
            Err(err) => return Err(Error::ConnectionError(err.to_string())),
        };
        let state = Rc::new(RefCell::new(State {
            windows: HashMap::new(),
        }));
        Ok(Application {
            connection: Rc::new(conn),
            screen_num,
            state,
        })
    }

    pub(crate) fn add_window(&self, id: u32, window: Rc<Window>) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.windows.insert(id, window);
        } else {
            log::warn!("Application::add_window - state already borrowed");
        }
    }

    fn remove_window(&self, id: u32) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.windows.remove(&id);
        } else {
            log::warn!("Application::remove_window - state already borrowed");
        }
    }

    fn window(&self, id: u32) -> Option<Rc<Window>> {
        if let Ok(state) = self.state.try_borrow() {
            state.windows.get(&id).cloned()
        } else {
            log::warn!("Application::window - state already borrowed");
            None
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
    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        loop {
            if let Some(ev) = self.connection.wait_for_event() {
                let ev_type = ev.response_type() & !0x80;
                match ev_type {
                    EXPOSE => {
                        let expose = unsafe { xcb::cast_event::<ExposeEvent>(&ev) };
                        let window_id = expose.window();
                        if let Some(w) = self.window(window_id) {
                            w.handle_expose(expose);
                        } else {
                            log::warn!("EXPOSE - failed to get window");
                        }
                    }
                    KEY_PRESS => {
                        let key_press = unsafe { xcb::cast_event::<KeyPressEvent>(&ev) };
                        let window_id = key_press.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_key_press(key_press);
                        } else {
                            log::warn!("KEY_PRESS - failed to get window");
                        }
                    }
                    BUTTON_PRESS => {
                        let button_press = unsafe { xcb::cast_event::<ButtonPressEvent>(&ev) };
                        let window_id = button_press.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_button_press(button_press);
                        } else {
                            log::warn!("BUTTON_PRESS - failed to get window");
                        }
                    }
                    BUTTON_RELEASE => {
                        let button_release = unsafe { xcb::cast_event::<ButtonReleaseEvent>(&ev) };
                        let window_id = button_release.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_button_release(button_release);
                        } else {
                            log::warn!("BUTTON_RELEASE - failed to get window");
                        }
                    }
                    MOTION_NOTIFY => {
                        let motion_notify = unsafe { xcb::cast_event::<MotionNotifyEvent>(&ev) };
                        let window_id = motion_notify.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_motion_notify(motion_notify);
                        } else {
                            log::warn!("MOTION_NOTIFY - failed to get window");
                        }
                    }
                    CLIENT_MESSAGE => {
                        let client_message = unsafe { xcb::cast_event::<ClientMessageEvent>(&ev) };
                        let window_id = client_message.window();
                        if let Some(w) = self.window(window_id) {
                            w.handle_client_message(client_message);
                        } else {
                            log::warn!("CLIENT_MESSAGE - failed to get window");
                        }
                    }
                    DESTROY_NOTIFY => {
                        let destroy_notify = unsafe { xcb::cast_event::<DestroyNotifyEvent>(&ev) };
                        let window_id = destroy_notify.window();
                        if let Some(w) = self.window(window_id) {
                            w.handle_destroy_notify(destroy_notify);
                        } else {
                            log::warn!("DESTROY_NOTIFY - failed to get window");
                        }
                        // Remove our reference to it and allow the Window to be dropped
                        self.remove_window(window_id);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn quit(&self) {
        // TODO(x11/quit): implement Application::quit
        //
        // We should loop over all the windows and destroy them to execute all the destroy handlers.
        //
        // Then we should send a custom client message event which would break out of the run loop.
        // Events seem to be only associated with windows so we will probably need
        // a message-only window that will act as the application level event driver.
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
