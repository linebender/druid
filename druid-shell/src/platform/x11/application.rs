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
        unimplemented!(); // TODO
    }

    pub fn get_locale() -> String {
        // TODO
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