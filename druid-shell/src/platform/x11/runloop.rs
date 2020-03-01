use std::collections::HashMap;

use super::application::Application;
use super::window::XWindow;

use crate::keycodes::KeyCode;
use std::time::{Duration, Instant};
pub struct RunLoop {
    // Used for forwarding events to the correct window, drawing, etc.
    x_id_to_xwindow_map: HashMap<u32, XWindow>,
}

impl RunLoop {
    pub fn new() -> RunLoop {
        RunLoop {
            x_id_to_xwindow_map: HashMap::new(),
        }
    }

    pub fn add_xwindow(&mut self, x_id: u32, xwindow: XWindow) {
        self.x_id_to_xwindow_map.insert(x_id, xwindow);
    }

    pub fn run(&mut self) {
        let conn = Application::get_connection();
        loop {
            if let Some(ev) = conn.wait_for_event() {
                let ev_type = ev.response_type() & !0x80;
                match ev_type {
                    xcb::EXPOSE => {
                        let event = unsafe { xcb::cast_event::<xcb::ExposeEvent>(&ev) };
                        let window_id = event.window();
                        println!("window_id {}", window_id);
                        self.x_id_to_xwindow_map
                            .get_mut(&window_id)
                            .map(|w| w.render());
                    }
                    xcb::KEY_PRESS => {
                        println!("key {}", ev_type);
                        let key_press: &xcb::KeyPressEvent = unsafe { xcb::cast_event(&ev) };
                        // println!("Key '{}' pressed ", key_press.detail());
                        let key: u32 = key_press.detail() as u32;
                        let key_code: KeyCode = key.into();
                        // println!("Key '{:?}' pressed ", key_code);

                        let window_id = key_press.event();
                        println!("window_id {}", window_id);
                        let v = self
                            .x_id_to_xwindow_map
                            .get_mut(&window_id)
                            .map(|w| w.key_down(key_code));
                    }
                    _ => {
                        println!("event {}", ev_type);
                    }
                }
            }
        }
    }
}
