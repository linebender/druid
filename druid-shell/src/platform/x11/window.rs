use std::any::Any;
use std::sync::Arc;
use std::convert::TryInto;

use xcb::ffi::XCB_COPY_FROM_PARENT;

use crate::runloop::RunLoop;
use crate::window::{IdleToken, Text, TimerToken, WinHandler};
use crate::mouse::{Cursor};
use crate::dialog::{FileDialogOptions, FileInfo};
use crate::kurbo::{Point, Size};
use crate::piet::{Piet, RenderContext};

use super::menu::Menu;
use super::error::Error;
use super::application::Application;

pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    size: Size,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        // TODO
        WindowBuilder {
            handler: None,
            title: String::new(),
            size: Size::new(500.0, 400.0),
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        unimplemented!(); // TODO
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        // TODO: currently a no-op
    }

    pub fn set_menu(&mut self, menu: Menu) {
        unimplemented!(); // TODO
    }

    pub fn build(self, run_loop: &mut RunLoop) -> Result<WindowHandle, Error> {
        // TODO: cascadeTopLeftFromPoint?
        // TODO: initWithFrame?
        // TODO: make menus?
        // TODO: multi-window support (???)
        // TODO: set double-buffering / present strategy?

        let conn = Application::get_connection();
        let screen_num = Application::get_screen_num();
        let window_id = conn.generate_id();
        let setup = conn.get_setup();
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = get_visual_from_screen(&screen).unwrap(); // TODO: don't unwrap here?
        let visual_id = visual_type.visual_id();

        // TODO: what window masks should go here?
        // TODO: event masks (mouse, keyboard)
        let cw_values = [
            (xcb::CW_BACK_PIXEL, screen.white_pixel()),
            (
                xcb::CW_EVENT_MASK,
                xcb::EVENT_MASK_EXPOSURE
                    | xcb::EVENT_MASK_KEY_PRESS
                    | xcb::EVENT_MASK_KEY_RELEASE
                    | xcb::EVENT_MASK_BUTTON_PRESS
                    | xcb::EVENT_MASK_BUTTON_RELEASE,
            ),
        ];

        // Create the actual window
        xcb::create_window(
            // Connection to the X server
            &conn,
            // Window depth -- TODO: what should go here?
            XCB_COPY_FROM_PARENT.try_into().unwrap(),
            // The new window's ID
            window_id,
            // Parent window of this new window
            // TODO: either `screen.root()` (no parent window) or pass parent here to attach
            screen.root(),
            // X-coordinate of the new window -- TODO: what to put here?
            0,
            // Y-coordinate of the new window -- TODO: what to put here?
            0,
            // Width of the new window -- TODO: figure out DPI scaling
            self.size.width as u16,
            // Height of the new window -- TODO: figure out DPI scaling
            self.size.height as u16,
            // Border width -- TODO: what to put here?
            0,
            // Window class type
            xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
            // Visual ID
            visual_id,
            // Window properties mask
            &cw_values,
        );

        // Set window title
        xcb::change_property(
            &conn,
            xcb::PROP_MODE_REPLACE as u8,
            window_id,
            xcb::ATOM_WM_NAME,
            xcb::ATOM_STRING,
            8,
            self.title.as_bytes(),
        );

        conn.flush();

        println!("THIS WINDOW'S ID: {}", window_id);

        let xwindow = XWindow::new(window_id, self.handler.unwrap());
        run_loop.add_xwindow(window_id, xwindow);

        Ok(WindowHandle::new(window_id))
    }
}

// X11-specific event handling and window drawing (etc.)
pub struct XWindow {
    window_id: u32,
    handler: Box<dyn WinHandler>,
}

impl XWindow {
    pub fn new(window_id: u32, window_handler: Box<dyn WinHandler>) -> XWindow {
        XWindow {
            window_id,
            handler: window_handler,
        }
    }

    pub fn debug_print(&self) {
        println!("XWindow::debug_print()");
    }

    pub fn paint(&mut self) {
        println!("XWindow::paint()");
        println!("window id: {}", self.window_id);
        let conn = Application::get_connection();
        let setup = conn.get_setup();
        let screen_num = Application::get_screen_num();
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = get_visual_from_screen(&screen).unwrap(); // TODO: don't unwrap here?
        let visual_id = visual_type.visual_id();

        // Create a draw surface
        let cairo_xcb_connection = unsafe {
            // TODO: difference between from_raw_none, from_raw_borrow, from_raw_full?
            cairo::XCBConnection::from_raw_none(conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t)
        };
        let cairo_drawable = cairo::XCBDrawable(self.window_id);
        let cairo_visual_type = unsafe {
            // TODO: difference between from_raw_none, from_raw_borrow, from_raw_full?
            cairo::XCBVisualType::from_raw_none(&mut visual_type.base as *mut _ as *mut cairo_sys::xcb_visualtype_t)
        };
        let cairo_surface = cairo::XCBSurface::create(
            &cairo_xcb_connection,
            &cairo_drawable,
            &cairo_visual_type,
            500, // TODO: don't hardcode size
            400, // TODO: don't hardcode size
        ).expect("couldn't create a cairo surface");

        let mut cairo_ctx = cairo::Context::new(&cairo_surface);
        cairo_ctx.set_source_rgb(0.0, 0.5, 0.0);
        cairo_ctx.paint();
        let mut piet_ctx = Piet::new(&mut cairo_ctx);
        self.handler.paint(&mut piet_ctx);
        if let Err(e) = piet_ctx.finish() {
            panic!("piet error on render: {:?}", e); // TODO: hook up to error or something?
        }
    }
}

// Apparently you have to get the visualtype this way :|
// TODO: consider refactoring
fn get_visual_from_screen(screen: &xcb::Screen<'_>) -> Option<xcb::xproto::Visualtype> {
    let conn = Application::get_connection();
    for depth in screen.allowed_depths() {
        for visual in depth.visuals() {
            if visual.visual_id() == screen.root_visual() {
                return Some(visual);
            }
        }
    }
    None
}

#[derive(Clone)]
pub struct IdleHandle;

impl IdleHandle {
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        unimplemented!(); // TODO
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        unimplemented!(); // TODO
    }
}

#[derive(Clone, Default)]
pub struct WindowHandle {
    window_id: u32,
}

impl WindowHandle {
    fn new(window_id: u32) -> WindowHandle {
        WindowHandle {
            window_id,
        }
    }

    pub fn show(&self) {
        let conn = Application::get_connection();
        xcb::map_window(conn.as_ref(), self.window_id);
        conn.as_ref().flush();
    }

    pub fn close(&self) {
        unimplemented!(); // TODO
    }

    pub fn bring_to_front_and_focus(&self) {
        unimplemented!(); // TODO
    }

    pub fn invalidate(&self) {
        unimplemented!(); // TODO
    }

    pub fn set_title(&self, title: &str) {
        unimplemented!(); // TODO
    }

    pub fn set_menu(&self, menu: Menu) {
        unimplemented!(); // TODO
    }

    pub fn text(&self) -> Text {
        unimplemented!(); // TODO
    }

    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        unimplemented!(); // TODO
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        unimplemented!(); // TODO
    }

    pub fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        unimplemented!(); // TODO
    }

    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        unimplemented!(); // TODO
    }

    pub fn show_context_menu(&self, menu: Menu, pos: Point) {
        unimplemented!(); // TODO
    }

    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        unimplemented!(); // TODO
    }

    pub fn get_dpi(&self) -> f32 {
        unimplemented!(); // TODO
    }
}