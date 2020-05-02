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

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::error::Error;
use super::window::Window;

#[derive(Clone)]
pub(crate) struct Application {
    connection: Rc<xcb::Connection>,
    screen_num: i32, // Needs a container when no longer const
    state: Rc<RefCell<State>>,
}

struct State {
    windows: HashMap<u32, Rc<Window>>,
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        let (conn, screen_num) = match xcb::Connection::connect_with_xlib_display() {
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
    pub(crate) fn connection(&self) -> &Rc<xcb::Connection> {
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
                    xcb::EXPOSE => {
                        let expose: &xcb::ExposeEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = expose.window();
                        if let Some(w) = self.window(window_id) {
                            w.handle_expose(expose);
                        } else {
                            log::warn!("EXPOSE - failed to get window");
                        }
                    }
                    xcb::KEY_PRESS => {
                        let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = key_press.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_key_press(key_press);
                        } else {
                            log::warn!("KEY_PRESS - failed to get window");
                        }
                    }
                    xcb::BUTTON_PRESS => {
                        let button_press: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = button_press.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_button_press(button_press);
                        } else {
                            log::warn!("BUTTON_PRESS - failed to get window");
                        }
                    }
                    xcb::BUTTON_RELEASE => {
                        let button_release: &xcb::ButtonReleaseEvent =
                            unsafe { xcb::cast_event(&ev) };
                        let window_id = button_release.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_button_release(button_release);
                        } else {
                            log::warn!("BUTTON_RELEASE - failed to get window");
                        }
                    }
                    xcb::MOTION_NOTIFY => {
                        let motion_notify: &xcb::MotionNotifyEvent =
                            unsafe { xcb::cast_event(&ev) };
                        let window_id = motion_notify.event();
                        if let Some(w) = self.window(window_id) {
                            w.handle_motion_notify(motion_notify);
                        } else {
                            log::warn!("MOTION_NOTIFY - failed to get window");
                        }
                    }
                    xcb::DESTROY_NOTIFY => {
                        let destroy_notify: &xcb::DestroyNotifyEvent =
                            unsafe { xcb::cast_event(&ev) };
                        let window_id = destroy_notify.window();
                        if let Some(w) = self.window(window_id) {
                            w.handle_destroy_notify(destroy_notify);
                        } else {
                            log::warn!("DESTROY_NOTIFY - failed to get window");
                        }
                        self.remove_window(window_id);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn quit(&self) {
        // TODO(x11/quit): implement Application::quit
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
