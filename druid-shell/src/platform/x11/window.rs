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
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::{Rc, Weak};

use cairo::XCBSurface;
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

pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    size: Size,
    min_size: Size,
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
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
        let conn = self.app.connection();
        let screen_num = self.app.screen_num();
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
            conn,
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
            conn,
            xcb::PROP_MODE_REPLACE as u8,
            window_id,
            xcb::ATOM_WM_NAME,
            xcb::ATOM_STRING,
            8,
            self.title.as_bytes(),
        );

        conn.flush();

        let handler = self.handler.unwrap();
        let window = Rc::new(Window::new(window_id, self.app.clone(), handler, self.size));
        let handle = WindowHandle::new(Rc::downgrade(&window));
        window.connect(handle.clone());

        self.app.add_window(window_id, window);

        Ok(handle)
    }
}

// X11-specific event handling and window drawing (etc.)
pub(crate) struct Window {
    id: u32,
    app: Application,
    handler: RefCell<Box<dyn WinHandler>>,
    cairo_context: RefCell<cairo::Context>,
    refresh_rate: Option<f64>,
    state: RefCell<WindowState>,
}

/// The mutable state of the window.
struct WindowState {
    size: Size,
}

impl Window {
    pub fn new(id: u32, app: Application, handler: Box<dyn WinHandler>, size: Size) -> Window {
        // Create a draw surface
        let setup = app.connection().get_setup();
        let screen_num = app.screen_num();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = get_visual_from_screen(&screen).unwrap();
        let cairo_xcb_connection = unsafe {
            cairo::XCBConnection::from_raw_none(
                app.connection().get_raw_conn() as *mut cairo_sys::xcb_connection_t
            )
        };
        let cairo_drawable = cairo::XCBDrawable(id);
        let cairo_visual_type = unsafe {
            cairo::XCBVisualType::from_raw_none(
                &mut visual_type.base as *mut _ as *mut cairo_sys::xcb_visualtype_t,
            )
        };
        let cairo_surface = XCBSurface::create(
            &cairo_xcb_connection,
            &cairo_drawable,
            &cairo_visual_type,
            size.width as i32,
            size.height as i32,
        )
        .expect("couldn't create a cairo surface");
        let cairo_context = RefCell::new(cairo::Context::new(&cairo_surface));

        // Figure out the refresh rate of the current screen
        let refresh_rate = util::refresh_rate(app.connection(), id);

        let handler = RefCell::new(handler);
        let state = RefCell::new(WindowState { size });
        Window {
            id,
            app,
            handler,
            cairo_context,
            refresh_rate,
            state,
        }
    }

    pub fn connect(&self, handle: WindowHandle) {
        let size = self.state.borrow().size;
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.connect(&handle.into());
            handler.size(size.width as u32, size.height as u32);
        } else {
            log::warn!("Window::connect - handler already borrowed");
        }
    }

    fn cairo_surface(&self) -> Result<XCBSurface, Error> {
        match self.cairo_context.try_borrow() {
            Ok(ctx) => match ctx.get_target().try_into() {
                Ok(surface) => Ok(surface),
                Err(err) => Err(Error::Generic(format!(
                    "try_into in Window::cairo_surface: {}",
                    err
                ))),
            },
            Err(_) => Err(Error::BorrowError(
                "cairo context in Window::cairo_surface".into(),
            )),
        }
    }

    fn set_size(&self, size: Size) {
        // TODO(x11/dpi_scaling): detect DPI and scale size
        let mut new_size = None;
        if let Ok(mut s) = self.state.try_borrow_mut() {
            if s.size != size {
                s.size = size;
                new_size = Some(size);
            }
        } else {
            log::warn!("Window::set_size - state already borrowed");
        }
        if let Some(size) = new_size {
            match self.cairo_surface() {
                Ok(surface) => {
                    if let Err(err) = surface.set_size(size.width as i32, size.height as i32) {
                        log::error!("Failed to update cairo surface size to {:?}: {}", size, err);
                    }
                }
                Err(err) => log::error!("Failed to get cairo surface: {}", err),
            }
            if let Ok(mut handler) = self.handler.try_borrow_mut() {
                handler.size(size.width as u32, size.height as u32);
            } else {
                log::warn!("Window::set_size - handler already borrowed");
            }
        }
    }

    fn request_redraw(&self, rect: Rect) {
        // See: http://rtbo.github.io/rust-xcb/xcb/ffi/xproto/struct.xcb_expose_event_t.html
        let expose_event = xcb::ExposeEvent::new(
            self.id,
            rect.x0 as u16,
            rect.y0 as u16,
            rect.width() as u16,
            rect.height() as u16,
            0,
        );
        xcb::send_event(
            self.app.connection(),
            false,
            self.id,
            xcb::EVENT_MASK_EXPOSURE,
            &expose_event,
        );
        self.app.connection().flush();
    }

    pub fn render(&self, invalid_rect: Rect) {
        // Figure out the window's current size
        let geometry_cookie = xcb::get_geometry(self.app.connection(), self.id);
        let reply = geometry_cookie.get_reply().unwrap();
        let size = Size::new(reply.width() as f64, reply.height() as f64);
        self.set_size(size);

        let mut anim = false;
        if let Ok(mut cairo_ctx) = self.cairo_context.try_borrow_mut() {
            let mut piet_ctx = Piet::new(&mut cairo_ctx);
            if let Ok(mut handler) = self.handler.try_borrow_mut() {
                anim = handler.paint(&mut piet_ctx, invalid_rect);
            } else {
                log::warn!("Window::render - handler already borrowed");
            }
            if let Err(e) = piet_ctx.finish() {
                // TODO(x11/errors): hook up to error or something?
                panic!("piet error on render: {:?}", e);
            }
        } else {
            log::warn!("Window::render - cairo context already borrowed");
        }

        if anim && self.refresh_rate.is_some() {
            // TODO(x11/render_improvements): Sleeping is a terrible way to schedule redraws.
            //     I think I'll end up having to write a redraw scheduler or something. :|
            //     Doing it this way for now to proof-of-concept it.
            let sleep_amount_ms = (1000.0 / self.refresh_rate.unwrap()) as u64;
            std::thread::sleep(std::time::Duration::from_millis(sleep_amount_ms));

            self.request_redraw(size.to_rect());
        }
        self.app.connection().flush();
    }

    pub fn key_down(&self, key_code: KeyCode) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.key_down(KeyEvent::new(
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
        } else {
            log::warn!("Window::key_down - handler already borrowed");
        }
    }

    pub fn mouse_down(&self, mouse_event: &MouseEvent) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.mouse_down(mouse_event);
        } else {
            log::warn!("Window::mouse_down - handler already borrowed");
        }
    }

    pub fn mouse_up(&self, mouse_event: &MouseEvent) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.mouse_up(mouse_event);
        } else {
            log::warn!("Window::mouse_up - handler already borrowed");
        }
    }

    pub fn mouse_move(&self, mouse_event: &MouseEvent) {
        if let Ok(mut handler) = self.handler.try_borrow_mut() {
            handler.mouse_move(mouse_event);
        } else {
            log::warn!("Window::mouse_move - handler already borrowed");
        }
    }

    pub fn show(&self) {
        xcb::map_window(self.app.connection(), self.id);
        self.app.connection().flush();
    }

    pub fn close(&self) {
        // Hopefully there aren't any references to this window after this function is called.
        xcb::destroy_window(self.app.connection(), self.id);
    }

    /// Set whether the window should be resizable
    pub fn resizable(&self, _resizable: bool) {
        log::warn!("Window::resizeable is currently unimplemented for X11 platforms.");
    }

    /// Set whether the window should show titlebar
    pub fn show_titlebar(&self, _show_titlebar: bool) {
        log::warn!("Window::show_titlebar is currently unimplemented for X11 platforms.");
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        // TODO(x11/misc): Unsure if this does exactly what the doc comment says; need a test case.
        xcb::configure_window(
            self.app.connection(),
            self.id,
            &[(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)],
        );
        xcb::set_input_focus(
            self.app.connection(),
            xcb::INPUT_FOCUS_POINTER_ROOT as u8,
            self.id,
            xcb::CURRENT_TIME,
        );
    }

    pub fn invalidate(&self) {
        let mut size = None;
        if let Ok(s) = self.state.try_borrow() {
            size = Some(s.size);
        } else {
            log::warn!("Window::invalidate - state already borrowed");
        }
        if let Some(size) = size {
            self.request_redraw(size.to_rect());
        }
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        self.request_redraw(rect);
    }

    pub fn set_title(&self, title: &str) {
        xcb::change_property(
            self.app.connection(),
            xcb::PROP_MODE_REPLACE as u8,
            self.id,
            xcb::ATOM_WM_NAME,
            xcb::ATOM_STRING,
            8,
            title.as_bytes(),
        );
    }

    pub fn set_menu(&self, _menu: Menu) {
        // TODO(x11/menus): implement Window::set_menu (currently a no-op)
    }

    pub fn get_dpi(&self) -> f32 {
        // TODO(x11/dpi_scaling): figure out DPI scaling
        96.0
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
pub(crate) struct IdleHandle;

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
pub(crate) struct WindowHandle {
    window: Weak<Window>,
}

impl WindowHandle {
    fn new(window: Weak<Window>) -> WindowHandle {
        WindowHandle { window }
    }

    pub fn show(&self) {
        if let Some(w) = self.window.upgrade() {
            w.show();
        }
    }

    pub fn close(&self) {
        if let Some(w) = self.window.upgrade() {
            w.close();
        }
    }

    pub fn resizable(&self, resizable: bool) {
        if let Some(w) = self.window.upgrade() {
            w.resizable(resizable);
        }
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        if let Some(w) = self.window.upgrade() {
            w.show_titlebar(show_titlebar);
        }
    }

    pub fn bring_to_front_and_focus(&self) {
        if let Some(w) = self.window.upgrade() {
            w.bring_to_front_and_focus();
        }
    }

    pub fn invalidate(&self) {
        if let Some(w) = self.window.upgrade() {
            w.invalidate();
        }
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(w) = self.window.upgrade() {
            w.invalidate_rect(rect);
        }
    }

    pub fn set_title(&self, title: &str) {
        if let Some(w) = self.window.upgrade() {
            w.set_title(title);
        }
    }

    pub fn set_menu(&self, menu: Menu) {
        if let Some(w) = self.window.upgrade() {
            w.set_menu(menu);
        }
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
        if let Some(w) = self.window.upgrade() {
            w.get_dpi()
        } else {
            96.0
        }
    }
}
