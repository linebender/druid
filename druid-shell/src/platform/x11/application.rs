// Copyright 2018 The xi-editor Authors.
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

use std::sync::Arc;

use super::clipboard::Clipboard;

pub struct Application;

impl Application {
    pub fn init() {
        // No-op.
    }

    pub fn quit() {
        // No-op.
    }

    pub fn get_connection() -> Arc<xcb::Connection> {
        XCB_CONNECTION.connection.clone()
    }

    pub fn get_screen_num() -> i32 {
        XCB_CONNECTION.screen_num
    }

    pub fn clipboard() -> Clipboard {
        // TODO(x11/clipboard): implement Application::clipboard
        unimplemented!();
    }

    pub fn get_locale() -> String {
        // TODO(x11/locales): implement Application::get_locale
        "en-US".into()
    }
}

struct XcbConnection {
    connection: Arc<xcb::Connection>,
    screen_num: i32,
}

impl XcbConnection {
    fn new() -> Self {
        let (conn, screen_num) = xcb::Connection::connect_with_xlib_display().unwrap();

        Self {
            connection: Arc::new(conn),
            screen_num,
        }
    }
}

lazy_static! {
    static ref XCB_CONNECTION: XcbConnection = XcbConnection::new();
}
