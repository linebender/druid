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
use std::sync::Arc;

use super::clipboard::Clipboard;
use super::window::XWindow;
use crate::application::AppHandler;
use crate::kurbo::Point;
use crate::{KeyCode, KeyModifiers, MouseButton, MouseEvent};

struct XcbConnection {
    connection: Arc<xcb::Connection>,
    screen_num: i32,
}

lazy_static! {
    static ref XCB_CONNECTION: XcbConnection = XcbConnection::new();
}

thread_local! {
    static WINDOW_MAP: RefCell<HashMap<u32, XWindow>> = RefCell::new(HashMap::new());
}

pub struct Application;

impl Application {
    pub fn new(_handler: Option<Box<dyn AppHandler>>) -> Application {
        Application
    }

    pub(crate) fn add_xwindow(id: u32, xwindow: XWindow) {
        WINDOW_MAP.with(|map| map.borrow_mut().insert(id, xwindow));
    }

    pub(crate) fn get_connection() -> Arc<xcb::Connection> {
        XCB_CONNECTION.connection_cloned()
    }

    pub(crate) fn get_screen_num() -> i32 {
        XCB_CONNECTION.screen_num()
    }

    pub fn run(&mut self) {
        let conn = XCB_CONNECTION.connection_cloned();
        loop {
            if let Some(ev) = conn.wait_for_event() {
                let ev_type = ev.response_type() & !0x80;
                //NOTE: I don't think we should be doing this here, but I'm trying to keep
                //the code mostly unchanged. My personal feeling is that the best approach
                //is to dispatch events to the window as early as possible, that is to say
                //I would send the *raw* events to the window and then let the window figure
                //out how to handle them. - @cmyr
                match ev_type {
                    xcb::EXPOSE => {
                        let expose: &xcb::ExposeEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = expose.window();
                        WINDOW_MAP.with(|map| {
                            let mut windows = map.borrow_mut();
                            windows.get_mut(&window_id).map(|w| w.render());
                        })
                    }
                    xcb::KEY_PRESS => {
                        let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&ev) };
                        let key: u32 = key_press.detail() as u32;
                        let key_code: KeyCode = key.into();

                        let window_id = key_press.event();
                        println!("window_id {}", window_id);
                        WINDOW_MAP.with(|map| {
                            let mut windows = map.borrow_mut();
                            windows.get_mut(&window_id).map(|w| w.key_down(key_code));
                        })
                    }
                    xcb::BUTTON_PRESS => {
                        let button_press: &xcb::ButtonPressEvent = unsafe { xcb::cast_event(&ev) };
                        let window_id = button_press.event();
                        let mouse_event = MouseEvent {
                            pos: Point::new(
                                button_press.event_x() as f64,
                                button_press.event_y() as f64,
                            ),
                            mods: KeyModifiers {
                                shift: false,
                                alt: false,
                                ctrl: false,
                                meta: false,
                            },
                            count: 0,
                            button: MouseButton::Left,
                        };
                        WINDOW_MAP.with(|map| {
                            let mut windows = map.borrow_mut();
                            windows
                                .get_mut(&window_id)
                                .map(|w| w.mouse_down(&mouse_event));
                        })
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn quit() {
        // No-op.
    }

    pub fn clipboard() -> Clipboard {
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

impl XcbConnection {
    fn new() -> Self {
        let (conn, screen_num) = xcb::Connection::connect_with_xlib_display().unwrap();

        Self {
            connection: Arc::new(conn),
            screen_num,
        }
    }

    fn connection_cloned(&self) -> Arc<xcb::Connection> {
        self.connection.clone()
    }

    fn screen_num(&self) -> i32 {
        self.screen_num
    }
}
