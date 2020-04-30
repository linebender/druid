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
use std::sync::Arc;

use crate::application::AppHandler;
use crate::kurbo::{Point, Rect};
use crate::{KeyCode, KeyModifiers, MouseButton, MouseButtons, MouseEvent};

use super::clipboard::Clipboard;
use super::error::Error;
use super::window::XWindow;

#[derive(Clone)]
pub(crate) struct Application {
    connection: Arc<xcb::Connection>,
    screen_num: i32, // Needs a container when no longer const
    state: Rc<RefCell<State>>,
}

struct State {
    // TODO: Figure out a better solution for window event passing,
    //       because this approach has reentrancy issues with window creation etc.
    windows: HashMap<u32, XWindow>,
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
            connection: Arc::new(conn),
            screen_num,
            state,
        })
    }

    pub(crate) fn add_window(&self, id: u32, xwindow: XWindow) {
        self.state.borrow_mut().windows.insert(id, xwindow);
    }

    #[allow(dead_code)]
    pub(crate) fn remove_window(&self, id: u32) {
        self.state.borrow_mut().windows.remove(&id);
    }

    pub(crate) fn connection(&self) -> &Arc<xcb::Connection> {
        &self.connection
    }

    pub(crate) fn screen_num(&self) -> i32 {
        self.screen_num
    }

    // TODO(x11/events): handle mouse scroll events
    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        loop {
            if let Some(ev) = self.connection.wait_for_event() {
                let ev_type = ev.response_type() & !0x80;
                // NOTE: I don't think we should be doing this here, but I'm trying to keep
                // the code mostly unchanged. My personal feeling is that the best approach
                // is to dispatch events to the window as early as possible, that is to say
                // I would send the *raw* events to the window and then let the window figure
                // out how to handle them. - @cmyr
                //
                // Can't get which window to send the raw event to without first parsing the event
                // and getting the window ID out of it :)  - @crsaracco
                match ev_type {
                    xcb::EXPOSE => {
                        let expose: &xcb::ExposeEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = expose.window();
                        // TODO(x11/dpi_scaling): when dpi scaling is
                        // implemented, it needs to be used here too
                        let rect = Rect::from_origin_size(
                            (expose.x() as f64, expose.y() as f64),
                            (expose.width() as f64, expose.height() as f64),
                        );
                        if let Ok(mut state) = self.state.try_borrow_mut() {
                            if let Some(w) = state.windows.get_mut(&window_id) {
                                w.render(rect);
                            }
                        } else {
                            log::warn!("Application state already borrowed");
                        }
                    }
                    xcb::KEY_PRESS => {
                        let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&ev) };
                        let key: u32 = key_press.detail() as u32;
                        let key_code: KeyCode = key.into();

                        let window_id = key_press.event();
                        println!("window_id {}", window_id);
                        if let Ok(mut state) = self.state.try_borrow_mut() {
                            if let Some(w) = state.windows.get_mut(&window_id) {
                                w.key_down(key_code);
                            }
                        } else {
                            log::warn!("Application state already borrowed");
                        }
                    }
                    xcb::BUTTON_PRESS => {
                        let button_press: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = button_press.event();
                        let mouse_event = MouseEvent {
                            pos: Point::new(
                                button_press.event_x() as f64,
                                button_press.event_y() as f64,
                            ),
                            // TODO: Fill with held down buttons
                            buttons: MouseButtons::new().with(MouseButton::Left),
                            mods: KeyModifiers {
                                shift: false,
                                alt: false,
                                ctrl: false,
                                meta: false,
                            },
                            count: 0,
                            button: MouseButton::Left,
                        };
                        if let Ok(mut state) = self.state.try_borrow_mut() {
                            if let Some(w) = state.windows.get_mut(&window_id) {
                                w.mouse_down(&mouse_event);
                            }
                        } else {
                            log::warn!("Application state already borrowed");
                        }
                    }
                    xcb::BUTTON_RELEASE => {
                        let button_release: &xcb::ButtonReleaseEvent =
                            unsafe { xcb::cast_event(&ev) };
                        let window_id = button_release.event();
                        let mouse_event = MouseEvent {
                            pos: Point::new(
                                button_release.event_x() as f64,
                                button_release.event_y() as f64,
                            ),
                            // TODO: Fill with held down buttons
                            buttons: MouseButtons::new(),
                            mods: KeyModifiers {
                                shift: false,
                                alt: false,
                                ctrl: false,
                                meta: false,
                            },
                            count: 0,
                            button: MouseButton::Left,
                        };
                        if let Ok(mut state) = self.state.try_borrow_mut() {
                            if let Some(w) = state.windows.get_mut(&window_id) {
                                w.mouse_up(&mouse_event);
                            }
                        } else {
                            log::warn!("Application state already borrowed");
                        }
                    }
                    xcb::MOTION_NOTIFY => {
                        let mouse_move: &xcb::MotionNotifyEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = mouse_move.event();
                        let mouse_event = MouseEvent {
                            pos: Point::new(
                                mouse_move.event_x() as f64,
                                mouse_move.event_y() as f64,
                            ),
                            // TODO: Fill with held down buttons
                            buttons: MouseButtons::new(),
                            mods: KeyModifiers {
                                shift: false,
                                alt: false,
                                ctrl: false,
                                meta: false,
                            },
                            count: 0,
                            button: MouseButton::None,
                        };
                        if let Ok(mut state) = self.state.try_borrow_mut() {
                            if let Some(w) = state.windows.get_mut(&window_id) {
                                w.mouse_move(&mouse_event);
                            }
                        } else {
                            log::warn!("Application state already borrowed");
                        }
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
