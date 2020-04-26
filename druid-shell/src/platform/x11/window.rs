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

//! X11 window creation and window management.

use std::any::Any;
use std::convert::TryInto;

use xcb::ffi::XCB_COPY_FROM_PARENT;

use crate::dialog::{FileDialogOptions, FileInfo};
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::keycodes::KeyCode;
use crate::kurbo::{Point, Rect, Size};
use crate::mouse::{Cursor, MouseEvent};
use crate::piet::{Piet, RenderContext};
use crate::window::{IdleToken, Text, TimerToken, WinHandler};

use super::application::Application;
use super::error::Error;
use super::menu::Menu;
use super::util;

pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    size: Size,
    min_size: Size,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            size: Size::new(500.0, 400.0),
            min_size: Size::new(0.0, 0.0),
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_min_size(&mut self, min_size: Size) {
        log::warn!("WindowBuilder::set_min_size is implemented, but the setting is currently unused for X11 platforms.");
        self.min_size = min_size;
    }

    pub fn resizable(&mut self, _resizable: bool) {
        log::warn!("WindowBuilder::resizable is currently unimplemented for X11 platforms.");
    }

    pub fn show_titlebar(&mut self, _show_titlebar: bool) {
        log::warn!("WindowBuilder::show_titlebar is currently unimplemented for X11 platforms.");
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, _menu: Menu) {
        // TODO(x11/menus): implement WindowBuilder::set_menu (currently a no-op)
    }

    // TODO(x11/menus): make menus if requested
    pub fn build(self) -> Result<WindowHandle, Error> {
        let conn = Application::get_connection();
        let screen_num = Application::get_screen_num();
        let window_id = conn.generate_id();
        let setup = conn.get_setup();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let visual_type = get_visual_from_screen(&screen).unwrap();
        let visual_id = visual_type.visual_id();

        let cw_values = [
            (xcb::CW_BACK_PIXEL, screen.white_pixel()),
            (
                xcb::CW_EVENT_MASK,
                xcb::EVENT_MASK_EXPOSURE
                    | xcb::EVENT_MASK_KEY_PRESS
                    | xcb::EVENT_MASK_KEY_RELEASE
                    | xcb::EVENT_MASK_BUTTON_PRESS
                    | xcb::EVENT_MASK_BUTTON_RELEASE
                    | xcb::EVENT_MASK_POINTER_MOTION,
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

        let mut handler = self.handler.unwrap();
        let handle = WindowHandle::new(window_id);
        handler.connect(&(handle.clone()).into());

        let xwindow = XWindow::new(window_id, handler, self.size);
        Application::add_xwindow(window_id, xwindow);

        Ok(handle)
    }
}

// X11-specific event handling and window drawing (etc.)
pub(crate) struct XWindow {
    window_id: u32,
    handler: Box<dyn WinHandler>,
    refresh_rate: Option<f64>,
}

impl XWindow {
    pub fn new(window_id: u32, window_handler: Box<dyn WinHandler>, size: Size) -> XWindow {
        let conn = Application::get_connection();

        // Figure out the refresh rate of the current screen
        let refresh_rate = util::refresh_rate(&conn, window_id);
        let mut xwindow = XWindow {
            window_id,
            handler: window_handler,
            refresh_rate,
        };
        // Let the window handler know the size of the window
        xwindow.communicate_size(size);

        xwindow
    }

    pub fn render(&mut self, invalid_rect: Rect) {
        let conn = Application::get_connection();
        let setup = conn.get_setup();
        let screen_num = Application::get_screen_num();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = get_visual_from_screen(&screen).unwrap();

        // Figure out the window's current size
        let geometry_cookie = xcb::get_geometry(&conn, self.window_id);
        let reply = geometry_cookie.get_reply().unwrap();
        let size = Size::new(reply.width() as f64, reply.height() as f64);
        self.communicate_size(size);

        // Create a draw surface
        // TODO(x11/render_improvements): We have to re-create this draw surface if the window size changes.
        let cairo_xcb_connection = unsafe {
            cairo::XCBConnection::from_raw_none(
                conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t
            )
        };
        let cairo_drawable = cairo::XCBDrawable(self.window_id);
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
        let mut cairo_context = cairo::Context::new(&cairo_surface);

        cairo_context.set_source_rgb(0.0, 0.0, 0.0);
        cairo_context.paint();
        let mut piet_ctx = Piet::new(&mut cairo_context);
        let anim = self.handler.paint(&mut piet_ctx, invalid_rect);
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

    pub fn mouse_down(&mut self, mouse_event: &MouseEvent) {
        self.handler.mouse_down(mouse_event);
    }

    pub fn mouse_up(&mut self, mouse_event: &MouseEvent) {
        self.handler.mouse_up(mouse_event);
    }

    pub fn mouse_move(&mut self, mouse_event: &MouseEvent) {
        self.handler.mouse_move(mouse_event);
    }

    fn communicate_size(&mut self, size: Size) {
        // TODO(x11/dpi_scaling): detect DPI and scale size
        self.handler.size(size.width as u32, size.height as u32);
    }
}

// Apparently you have to get the visualtype this way :|
fn get_visual_from_screen(screen: &xcb::Screen<'_>) -> Option<xcb::xproto::Visualtype> {
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
    pub fn add_idle_callback<F>(&self, _callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        // TODO(x11/idle_handles): implement IdleHandle::add_idle_callback
        log::warn!("IdleHandle::add_idle_callback is currently unimplemented for X11 platforms.");
    }

    pub fn add_idle_token(&self, _token: IdleToken) {
        // TODO(x11/idle_handles): implement IdleHandle::add_idle_token
        log::warn!("IdleHandle::add_idle_token is currently unimplemented for X11 platforms.");
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
        // Hopefully there aren't any references to this window after this function is called.
        let conn = Application::get_connection();
        xcb::destroy_window(&conn, self.window_id);
    }

    /// Set whether the window should be resizable
    pub fn resizable(&self, _resizable: bool) {
        log::warn!("WindowHandle::resizeable is currently unimplemented for X11 platforms.");
    }

    /// Set whether the window should show titlebar
    pub fn show_titlebar(&self, _show_titlebar: bool) {
        log::warn!("WindowHandle::show_titlebar is currently unimplemented for X11 platforms.");
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        // TODO(x11/misc): Unsure if this does exactly what the doc comment says; need a test case.
        let conn = Application::get_connection();
        xcb::configure_window(
            &conn,
            self.window_id,
            &[(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)],
        );
        xcb::set_input_focus(
            &conn,
            xcb::INPUT_FOCUS_POINTER_ROOT as u8,
            self.window_id,
            xcb::CURRENT_TIME,
        );
    }

    pub fn invalidate(&self) {
        request_redraw(self.window_id);
    }

    pub fn invalidate_rect(&self, _rect: Rect) {
        // TODO(x11/render_improvements): set the bounds correctly.
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

    pub fn set_menu(&self, _menu: Menu) {
        // TODO(x11/menus): implement WindowHandle::set_menu (currently a no-op)
    }

    pub fn text(&self) -> Text {
        // I'm not entirely sure what this method is doing here, so here's a Text.
        Text::new()
    }

    pub fn request_timer(&self, _deadline: std::time::Instant) -> TimerToken {
        // TODO(x11/timers): implement WindowHandle::request_timer
        //     This one might be tricky, since there's not really any timers to hook into in X11.
        //     Might have to code up our own Timer struct, running in its own thread?
        log::warn!("WindowHandle::resizeable is currently unimplemented for X11 platforms.");
        TimerToken::INVALID
    }

    pub fn set_cursor(&mut self, _cursor: &Cursor) {
        // TODO(x11/cursors): implement WindowHandle::set_cursor
        log::warn!("WindowHandle::set_cursor is currently unimplemented for X11 platforms.");
    }

    pub fn open_file_sync(&mut self, _options: FileDialogOptions) -> Option<FileInfo> {
        // TODO(x11/file_dialogs): implement WindowHandle::open_file_sync
        log::warn!("WindowHandle::open_file_sync is currently unimplemented for X11 platforms.");
        None
    }

    pub fn save_as_sync(&mut self, _options: FileDialogOptions) -> Option<FileInfo> {
        // TODO(x11/file_dialogs): implement WindowHandle::save_as_sync
        log::warn!("WindowHandle::save_as_sync is currently unimplemented for X11 platforms.");
        None
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        // TODO(x11/menus): implement WindowHandle::show_context_menu
        log::warn!("WindowHandle::show_context_menu is currently unimplemented for X11 platforms.");
    }

    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        // TODO(x11/idle_handles): implement WindowHandle::get_idle_handle
        //log::warn!("WindowHandle::get_idle_handle is currently unimplemented for X11 platforms.");
        Some(IdleHandle)
    }

    pub fn get_dpi(&self) -> f32 {
        // TODO(x11/dpi_scaling): figure out DPI scaling
        log::warn!("WindowHandle::get_dpi is currently unimplemented for X11 platforms.");
        96.0
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
    conn.flush();
}
