use std::any::Any;
use std::convert::TryInto;
use std::sync::Arc;

use xcb::ffi::XCB_COPY_FROM_PARENT;

use crate::dialog::{FileDialogOptions, FileInfo};
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::keycodes::KeyCode;
use crate::kurbo::{Point, Size};
use crate::mouse::Cursor;
use crate::piet::{Piet, RenderContext};
use crate::runloop::RunLoop;
use crate::window::{IdleToken, Text, TimerToken, WinHandler};

use super::application::Application;
use super::error::Error;
use super::menu::Menu;
use super::util;

pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    size: Size,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
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
        self.size = size;
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        // TODO(x11/menus): implement WindowBuilder::set_menu (currently a no-op)
    }

    // TODO(x11/menus): make menus if requested
    pub fn build(self, run_loop: &mut RunLoop) -> Result<WindowHandle, Error> {
        let conn = Application::get_connection();
        let screen_num = Application::get_screen_num();
        let window_id = conn.generate_id();
        let setup = conn.get_setup();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = get_visual_from_screen(&screen).unwrap();
        let visual_id = visual_type.visual_id();

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
            // Window depth
            XCB_COPY_FROM_PARENT.try_into().unwrap(),
            // The new window's ID
            window_id,
            // Parent window of this new window
            // TODO(#468): either `screen.root()` (no parent window) or pass parent here to attach
            screen.root(),
            // X-coordinate of the new window
            0,
            // Y-coordinate of the new window
            0,
            // Width of the new window
            // TODO(x11/dpi_scaling): figure out DPI scaling
            self.size.width as u16,
            // Height of the new window
            // TODO(x11/dpi_scaling): figure out DPI scaling
            self.size.height as u16,
            // Border width
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

        let xwindow = XWindow::new(window_id, self.handler.unwrap(), self.size);
        run_loop.add_xwindow(window_id, xwindow);

        Ok(WindowHandle::new(window_id))
    }
}

// X11-specific event handling and window drawing (etc.)
pub struct XWindow {
    window_id: u32,
    handler: Box<dyn WinHandler>,
    cairo_context: cairo::Context,
    refresh_rate: Option<f64>,
}

impl XWindow {
    pub fn new(window_id: u32, window_handler: Box<dyn WinHandler>, size: Size) -> XWindow {
        let conn = Application::get_connection();
        let setup = conn.get_setup();
        let screen_num = Application::get_screen_num();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = get_visual_from_screen(&screen).unwrap();

        // Create a draw surface
        // TODO(x11/initial_pr): We have to re-create this draw surface if the window size changes.
        let cairo_xcb_connection = unsafe {
            cairo::XCBConnection::from_raw_none(
                conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t
            )
        };
        let cairo_drawable = cairo::XCBDrawable(window_id);
        let cairo_visual_type = unsafe {
            cairo::XCBVisualType::from_raw_none(
                &mut visual_type.base as *mut _ as *mut cairo_sys::xcb_visualtype_t,
            )
        };
        let cairo_surface = cairo::XCBSurface::create(
            &cairo_xcb_connection,
            &cairo_drawable,
            &cairo_visual_type,
            size.width as i32,
            size.height as i32,
        )
        .expect("couldn't create a cairo surface");
        let mut cairo_ctx = cairo::Context::new(&cairo_surface);

        // Figure out the refresh rate of the current screen
        let refresh_rate = util::refresh_rate(&conn, window_id);

        XWindow {
            window_id,
            handler: window_handler,
            cairo_context: cairo_ctx,
            refresh_rate,
        }
    }

    pub fn render(&mut self) {
        //println!("XWindow::paint()");
        //println!("window id: {}", self.window_id);
        let conn = Application::get_connection();

        self.cairo_context.set_source_rgb(0.0, 0.0, 0.0);
        self.cairo_context.paint();
        let mut piet_ctx = Piet::new(&mut self.cairo_context);
        let anim = self.handler.paint(&mut piet_ctx);
        if let Err(e) = piet_ctx.finish() {
            // TODO(x11/errors): hook up to error or something?
            panic!("piet error on render: {:?}", e);
        }

        if anim && self.refresh_rate.is_some() {
            // TODO(x11/render_improvements): Sleeping is a terrible way to schedule redraws.
            //     I think I'll end up having to write a redraw scheduler or something. :|
            //     Doing it this way for now to proof-of-concept it.
            let sleep_amount_ms = (1000.0 / self.refresh_rate.unwrap()) as u64;
            std::thread::sleep(std::time::Duration::from_millis(sleep_amount_ms));

            request_redraw(self.window_id);
        }
        conn.flush();
    }

    pub fn key_down(&mut self, key_code: KeyCode) {
        println!("XWindow::key_down()");

        self.handler.key_down(KeyEvent::new(
            key_code,
            false,
            KeyModifiers {
                /// Shift.
                shift: false,
                /// Option on macOS.
                alt: false,
                /// Control.
                ctrl: false,
                /// Meta / Windows / Command
                meta: false,
            },
            key_code,
            key_code,
        ));
    }
}

// Apparently you have to get the visualtype this way :|
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
        // TODO(x11/idle_handles): implement IdleHandle::add_idle_callback
        unimplemented!();
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        // TODO(x11/idle_handles): implement IdleHandle::add_idle_token
        unimplemented!();
    }
}

#[derive(Clone, Default)]
pub struct WindowHandle {
    window_id: u32,
}

impl WindowHandle {
    fn new(window_id: u32) -> WindowHandle {
        WindowHandle { window_id }
    }

    pub fn show(&self) {
        let conn = Application::get_connection();
        xcb::map_window(conn.as_ref(), self.window_id);
        conn.as_ref().flush();
    }

    pub fn close(&self) {
        // TODO(x11/initial_pr): implement WindowHandle::close, or re-TODO
        unimplemented!();
    }

    pub fn bring_to_front_and_focus(&self) {
        // TODO(x11/initial_pr): implement WindowHandle::bring_to_front_and_focus, or re-TODO
        unimplemented!();
    }

    pub fn invalidate(&self) {
        request_redraw(self.window_id);
    }

    pub fn set_title(&self, title: &str) {
        let conn = Application::get_connection();
        xcb::change_property(
            &conn,
            xcb::PROP_MODE_REPLACE as u8,
            self.window_id,
            xcb::ATOM_WM_NAME,
            xcb::ATOM_STRING,
            8,
            title.as_bytes(),
        );
    }

    pub fn set_menu(&self, menu: Menu) {
        // TODO(x11/menus): implement WindowHandle::set_menu (currently a no-op)
    }

    pub fn text(&self) -> Text {
        // I'm not entirely sure what this method is doing here, so here's a Text.
        Text::new()
    }

    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        // TODO(x11/timers): implement WindowHandle::request_timer
        //     This one might be tricky, since there's not really any timers to hook into in X11.
        //     Might have to code up our own Timer struct, running in its own thread?
        unimplemented!();
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        // TODO(x11/cursors): implement WindowHandle::set_cursor
        unimplemented!();
    }

    pub fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        // TODO(x11/file_dialogs): implement WindowHandle::open_file_sync
        unimplemented!();
    }

    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        // TODO(x11/file_dialogs): implement WindowHandle::save_as_sync
        unimplemented!();
    }

    pub fn show_context_menu(&self, menu: Menu, pos: Point) {
        // TODO(x11/menus): implement WindowHandle::show_context_menu
        unimplemented!();
    }

    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        // TODO(x11/idle_handles): implement WindowHandle::get_idle_handle
        unimplemented!();
    }

    pub fn get_dpi(&self) -> f32 {
        // TODO(x11/dpi_scaling): figure out DPI scaling
        unimplemented!();
    }
}

fn request_redraw(window_id: u32) {
    let conn = Application::get_connection();

    // TODO(x11/render_improvements): Set x, y, width, and height correctly.
    //     We redraw the entire surface on an ExposeEvent, so these args currently do nothing.
    // See: http://rtbo.github.io/rust-xcb/xcb/ffi/xproto/struct.xcb_expose_event_t.html
    let expose_event = xcb::ExposeEvent::new(
        window_id, /*x=*/ 0, /*y=*/ 0, /*width=*/ 0, /*height=*/ 0,
        /*count=*/ 0,
    );
    xcb::send_event(
        &conn,
        false,
        window_id,
        xcb::EVENT_MASK_EXPOSURE,
        &expose_event,
    );
}
