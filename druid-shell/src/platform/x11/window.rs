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

use cairo::{XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use xcb::{
    Atom, Visualtype, ATOM_ATOM, ATOM_STRING, ATOM_WM_NAME, CONFIG_WINDOW_STACK_MODE,
    COPY_FROM_PARENT, CURRENT_TIME, CW_BACK_PIXEL, CW_EVENT_MASK, EVENT_MASK_BUTTON_PRESS,
    EVENT_MASK_BUTTON_RELEASE, EVENT_MASK_EXPOSURE, EVENT_MASK_KEY_PRESS, EVENT_MASK_KEY_RELEASE,
    EVENT_MASK_POINTER_MOTION, EVENT_MASK_STRUCTURE_NOTIFY, INPUT_FOCUS_POINTER_ROOT,
    PROP_MODE_REPLACE, STACK_MODE_ABOVE, WINDOW_CLASS_INPUT_OUTPUT,
};

use crate::dialog::{FileDialogOptions, FileInfo};
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::keycodes::KeyCode;
use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
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

    /// Registers and returns all the atoms that the window will need.
    fn atoms(&self, window_id: u32) -> Result<WindowAtoms, Error> {
        let conn = self.app.connection();

        // WM_PROTOCOLS
        //
        // List of atoms that identify the communications protocols between
        // the client and window manager in which the client is willing to participate.
        //
        // https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#wm_protocols_property
        let wm_protocols = match xcb::intern_atom(conn, false, "WM_PROTOCOLS").get_reply() {
            Ok(reply) => reply.atom(),
            Err(err) => {
                return Err(Error::Generic(format!(
                    "failed to get WM_PROTOCOLS: {}",
                    err
                )))
            }
        };

        // WM_DELETE_WINDOW
        //
        // Including this atom in the WM_PROTOCOLS property on each window makes sure that
        // if the window manager respects WM_DELETE_WINDOW it will send us the event.
        //
        // The WM_DELETE_WINDOW event is sent when there is a request to close the window.
        // Registering for but ignoring this event means that the window will remain open.
        //
        // https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#window_deletion
        let wm_delete_window = match xcb::intern_atom(conn, false, "WM_DELETE_WINDOW").get_reply() {
            Ok(reply) => reply.atom(),
            Err(err) => {
                return Err(Error::Generic(format!(
                    "failed to get WM_DELETE_WINDOW: {}",
                    err
                )))
            }
        };

        // Replace the window's WM_PROTOCOLS with the following.
        let protocols = [wm_delete_window];
        // TODO(x11/errors): Check the response for errors?
        xcb::change_property(
            conn,
            PROP_MODE_REPLACE as u8,
            window_id,
            wm_protocols,
            ATOM_ATOM,
            32,
            &protocols,
        );

        Ok(WindowAtoms {
            wm_protocols,
            wm_delete_window,
        })
    }

    /// Create a new cairo `Context`.
    fn create_cairo_context(
        &self,
        window_id: u32,
        visual_type: &mut Visualtype,
    ) -> Result<RefCell<cairo::Context>, Error> {
        let conn = self.app.connection();
        let cairo_xcb_connection = unsafe {
            XCBConnection::from_raw_none(conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t)
        };
        let cairo_drawable = XCBDrawable(window_id);
        let cairo_visual_type = unsafe {
            XCBVisualType::from_raw_none(
                &mut visual_type.base as *mut _ as *mut cairo_sys::xcb_visualtype_t,
            )
        };
        let cairo_surface = XCBSurface::create(
            &cairo_xcb_connection,
            &cairo_drawable,
            &cairo_visual_type,
            self.size.width as i32,
            self.size.height as i32,
        );
        match cairo_surface {
            Ok(cairo_surface) => Ok(RefCell::new(cairo::Context::new(&cairo_surface))),
            Err(err) => Err(Error::Generic(format!(
                "Failed to create cairo surface: {}",
                err
            ))),
        }
    }

    // TODO(x11/menus): make menus if requested
    pub fn build(self) -> Result<WindowHandle, Error> {
        let conn = self.app.connection();
        let screen_num = self.app.screen_num();
        let id = conn.generate_id();
        let setup = conn.get_setup();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let mut visual_type = util::get_visual_from_screen(&screen).unwrap();
        let visual_id = visual_type.visual_id();

        let cw_values = [
            (CW_BACK_PIXEL, screen.white_pixel()),
            (
                CW_EVENT_MASK,
                EVENT_MASK_EXPOSURE
                    | EVENT_MASK_STRUCTURE_NOTIFY
                    | EVENT_MASK_KEY_PRESS
                    | EVENT_MASK_KEY_RELEASE
                    | EVENT_MASK_BUTTON_PRESS
                    | EVENT_MASK_BUTTON_RELEASE
                    | EVENT_MASK_POINTER_MOTION,
            ),
        ];

        // Create the actual window
        // TODO(x11/errors): check that this actually succeeds?
        xcb::create_window(
            // Connection to the X server
            conn,
            // Window depth
            COPY_FROM_PARENT.try_into().unwrap(),
            // The new window's ID
            id,
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
            WINDOW_CLASS_INPUT_OUTPUT as u16,
            // Visual ID
            visual_id,
            // Window properties mask
            &cw_values,
        );

        // TODO(x11/errors): Should do proper cleanup (window destruction etc) in case of error
        let atoms = self.atoms(id)?;
        let cairo_context = self.create_cairo_context(id, &mut visual_type)?;
        // Figure out the refresh rate of the current screen
        let refresh_rate = util::refresh_rate(conn, id);
        let state = RefCell::new(WindowState { size: self.size });
        let handler = RefCell::new(self.handler.unwrap());

        let window = Rc::new(Window {
            id,
            app: self.app.clone(),
            handler,
            cairo_context,
            refresh_rate,
            atoms,
            state,
        });
        window.set_title(&self.title);

        let handle = WindowHandle::new(id, Rc::downgrade(&window));
        window.connect(handle.clone())?;

        self.app.add_window(id, window)?;

        Ok(handle)
    }
}

/// An X11 window.
pub(crate) struct Window {
    id: u32,
    app: Application,
    handler: RefCell<Box<dyn WinHandler>>,
    cairo_context: RefCell<cairo::Context>,
    refresh_rate: Option<f64>,
    atoms: WindowAtoms,
    state: RefCell<WindowState>,
}

/// All the Atom references the window needs.
struct WindowAtoms {
    wm_protocols: Atom,
    wm_delete_window: Atom,
}

/// The mutable state of the window.
struct WindowState {
    size: Size,
}

impl Window {
    fn connect(&self, handle: WindowHandle) -> Result<(), Error> {
        match self.handler.try_borrow_mut() {
            Ok(mut handler) => {
                let size = self.size()?;
                handler.connect(&handle.into());
                handler.size(size.width as u32, size.height as u32);
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Window::connect handler: {}",
                err
            ))),
        }
    }

    /// Start the destruction of the window.
    ///
    /// ### Connection
    ///
    /// Does not flush the connection.
    pub fn destroy(&self) {
        xcb::destroy_window(self.app.connection(), self.id);
    }

    fn cairo_surface(&self) -> Result<XCBSurface, Error> {
        match self.cairo_context.try_borrow() {
            Ok(ctx) => match ctx.get_target().try_into() {
                Ok(surface) => Ok(surface),
                Err(err) => Err(Error::Generic(format!(
                    "Window::cairo_surface ctx.try_into: {}",
                    err
                ))),
            },
            Err(err) => Err(Error::BorrowError(format!(
                "Window::cairo_surface cairo_context: {}",
                err
            ))),
        }
    }

    fn size(&self) -> Result<Size, Error> {
        match self.state.try_borrow() {
            Ok(state) => Ok(state.size),
            Err(err) => Err(Error::BorrowError(format!("Window::size state: {}", err))),
        }
    }

    fn set_size(&self, size: Size) -> Result<(), Error> {
        // TODO(x11/dpi_scaling): detect DPI and scale size
        let new_size = match self.state.try_borrow_mut() {
            Ok(mut state) => {
                if state.size != size {
                    state.size = size;
                    Some(size)
                } else {
                    None
                }
            }
            Err(err) => {
                return Err(Error::BorrowError(format!(
                    "Window::set_size state: {}",
                    err
                )))
            }
        };
        if let Some(size) = new_size {
            let cairo_surface = self.cairo_surface()?;
            if let Err(err) = cairo_surface.set_size(size.width as i32, size.height as i32) {
                return Err(Error::Generic(format!(
                    "Failed to update cairo surface size to {:?}: {}",
                    size, err
                )));
            }
            match self.handler.try_borrow_mut() {
                Ok(mut handler) => handler.size(size.width as u32, size.height as u32),
                Err(err) => {
                    return Err(Error::BorrowError(format!(
                        "Window::set_size handler: {}",
                        err
                    )))
                }
            }
        }
        Ok(())
    }

    /// Tell the X server to mark the specified `rect` as needing redraw.
    ///
    /// ### Connection
    ///
    /// Does not flush the connection.
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
            EVENT_MASK_EXPOSURE,
            &expose_event,
        );
    }

    fn render(&self, invalid_rect: Rect) -> Result<(), Error> {
        // TODO(x11/errors): this function should return a an error
        // instead of panicking or logging if the error isn't recoverable.

        // Figure out the window's current size
        let geometry_cookie = xcb::get_geometry(self.app.connection(), self.id);
        let reply = geometry_cookie.get_reply().unwrap();
        let size = Size::new(reply.width() as f64, reply.height() as f64);
        self.set_size(size)?;

        let mut anim = false;
        if let Ok(mut cairo_ctx) = self.cairo_context.try_borrow_mut() {
            let mut piet_ctx = Piet::new(&mut cairo_ctx);
            piet_ctx.clip(invalid_rect);
            if let Ok(mut handler) = self.handler.try_borrow_mut() {
                anim = handler.paint(&mut piet_ctx, invalid_rect);
            } else {
                log::error!("Window::render - handler already borrowed");
            }
            if let Err(e) = piet_ctx.finish() {
                log::error!("Window::render - piet finish failed: {}", e);
            }
            cairo_ctx.reset_clip();
        } else {
            log::error!("Window::render - cairo context already borrowed");
        }

        if anim && self.refresh_rate.is_some() {
            // TODO(x11/render_improvements): Sleeping is a terrible way to schedule redraws.
            //     I think I'll end up having to write a redraw scheduler or something. :|
            //     Doing it this way for now to proof-of-concept it.
            //
            // Eventually we also need to make sure we respect V-Sync timings.
            // A druid-shell test utility should probably be written to verify that.
            // Inspiration can be taken from: https://www.vsynctester.com/we
            let sleep_amount_ms = (1000.0 / self.refresh_rate.unwrap()) as u64;
            std::thread::sleep(std::time::Duration::from_millis(sleep_amount_ms));

            self.request_redraw(size.to_rect());
        }
        self.app.connection().flush();
        Ok(())
    }

    fn show(&self) {
        xcb::map_window(self.app.connection(), self.id);
        self.app.connection().flush();
    }

    fn close(&self) {
        self.destroy();
        self.app.connection().flush();
    }

    /// Set whether the window should be resizable
    fn resizable(&self, _resizable: bool) {
        log::warn!("Window::resizeable is currently unimplemented for X11 platforms.");
    }

    /// Set whether the window should show titlebar
    fn show_titlebar(&self, _show_titlebar: bool) {
        log::warn!("Window::show_titlebar is currently unimplemented for X11 platforms.");
    }

    /// Bring this window to the front of the window stack and give it focus.
    fn bring_to_front_and_focus(&self) {
        // TODO(x11/misc): Unsure if this does exactly what the doc comment says; need a test case.
        xcb::configure_window(
            self.app.connection(),
            self.id,
            &[(CONFIG_WINDOW_STACK_MODE as u16, STACK_MODE_ABOVE)],
        );
        xcb::set_input_focus(
            self.app.connection(),
            INPUT_FOCUS_POINTER_ROOT as u8,
            self.id,
            CURRENT_TIME,
        );
        self.app.connection().flush();
    }

    fn invalidate(&self) {
        match self.size() {
            Ok(size) => self.invalidate_rect(size.to_rect()),
            Err(err) => log::error!("Window::invalidate - failed to get size: {}", err),
        }
    }

    fn invalidate_rect(&self, rect: Rect) {
        self.request_redraw(rect);
        self.app.connection().flush();
    }

    fn set_title(&self, title: &str) {
        xcb::change_property(
            self.app.connection(),
            PROP_MODE_REPLACE as u8,
            self.id,
            ATOM_WM_NAME,
            ATOM_STRING,
            8,
            title.as_bytes(),
        );
        self.app.connection().flush();
    }

    fn set_menu(&self, _menu: Menu) {
        // TODO(x11/menus): implement Window::set_menu (currently a no-op)
    }

    fn get_dpi(&self) -> f32 {
        // TODO(x11/dpi_scaling): figure out DPI scaling
        96.0
    }

    pub fn handle_expose(&self, expose: &xcb::ExposeEvent) -> Result<(), Error> {
        // TODO(x11/dpi_scaling): when dpi scaling is
        // implemented, it needs to be used here too
        let rect = Rect::from_origin_size(
            (expose.x() as f64, expose.y() as f64),
            (expose.width() as f64, expose.height() as f64),
        );
        self.render(rect)
    }

    pub fn handle_key_press(&self, key_press: &xcb::KeyPressEvent) -> Result<(), Error> {
        let key: u32 = key_press.detail() as u32;
        let key_code: KeyCode = key.into();
        let key_event = KeyEvent::new(
            key_code,
            // TODO: detect repeated keys
            false,
            key_mods(key_press.state()),
            key_code,
            key_code,
        );
        match self.handler.try_borrow_mut() {
            Ok(mut handler) => {
                handler.key_down(key_event);
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Window::handle_key_press handle: {}",
                err
            ))),
        }
    }

    pub fn handle_button_press(&self, button_press: &xcb::ButtonPressEvent) -> Result<(), Error> {
        let button = mouse_button(button_press.detail());
        let mouse_event = MouseEvent {
            pos: Point::new(button_press.event_x() as f64, button_press.event_y() as f64),
            // The xcb state field doesn't include the newly pressed button, but
            // druid wants it to be included.
            buttons: mouse_buttons(button_press.state()).with(button),
            mods: key_mods(button_press.state()),
            // TODO: detect the count
            count: 1,
            focus: false,
            button,
            wheel_delta: Vec2::ZERO,
        };
        match self.handler.try_borrow_mut() {
            Ok(mut handler) => {
                handler.mouse_down(&mouse_event);
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Window::handle_button_press handle: {}",
                err
            ))),
        }
    }

    pub fn handle_button_release(
        &self,
        button_release: &xcb::ButtonReleaseEvent,
    ) -> Result<(), Error> {
        let button = mouse_button(button_release.detail());
        let mouse_event = MouseEvent {
            pos: Point::new(
                button_release.event_x() as f64,
                button_release.event_y() as f64,
            ),
            // The xcb state includes the newly released button, but druid
            // doesn't want it.
            buttons: mouse_buttons(button_release.state()).without(button),
            mods: key_mods(button_release.state()),
            count: 0,
            focus: false,
            button,
            wheel_delta: Vec2::ZERO,
        };
        match self.handler.try_borrow_mut() {
            Ok(mut handler) => {
                handler.mouse_up(&mouse_event);
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Window::handle_button_release handle: {}",
                err
            ))),
        }
    }

    pub fn handle_motion_notify(
        &self,
        motion_notify: &xcb::MotionNotifyEvent,
    ) -> Result<(), Error> {
        let mouse_event = MouseEvent {
            pos: Point::new(
                motion_notify.event_x() as f64,
                motion_notify.event_y() as f64,
            ),
            buttons: mouse_buttons(motion_notify.state()),
            mods: key_mods(motion_notify.state()),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        };
        match self.handler.try_borrow_mut() {
            Ok(mut handler) => {
                handler.mouse_move(&mouse_event);
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Window::handle_motion_notify handle: {}",
                err
            ))),
        }
    }

    pub fn handle_client_message(
        &self,
        client_message: &xcb::ClientMessageEvent,
    ) -> Result<(), Error> {
        // https://www.x.org/releases/X11R7.7/doc/libX11/libX11/libX11.html#id2745388
        // https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#window_deletion
        if client_message.type_() == self.atoms.wm_protocols && client_message.format() == 32 {
            let protocol = client_message.data().data32()[0];
            if protocol == self.atoms.wm_delete_window {
                self.close();
            }
        }
        Ok(())
    }

    pub fn handle_destroy_notify(
        &self,
        _destroy_notify: &xcb::DestroyNotifyEvent,
    ) -> Result<(), Error> {
        match self.handler.try_borrow_mut() {
            Ok(mut handler) => {
                handler.destroy();
                Ok(())
            }
            Err(err) => Err(Error::BorrowError(format!(
                "Window::handle_destroy_notify handle: {}",
                err
            ))),
        }
    }
}

// Converts from, e.g., the `details` field of `xcb::xproto::ButtonPressEvent`
fn mouse_button(button: u8) -> MouseButton {
    match button {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        // buttons 4 through 7 are for scrolling.
        4..=7 => MouseButton::None,
        8 => MouseButton::X1,
        9 => MouseButton::X2,
        _ => {
            log::warn!("unknown mouse button code {}", button);
            MouseButton::None
        }
    }
}

// Extracts the mouse buttons from, e.g., the `state` field of
// `xcb::xproto::ButtonPressEvent`
fn mouse_buttons(mods: u16) -> MouseButtons {
    let mut buttons = MouseButtons::new();
    let button_masks = &[
        (xcb::xproto::BUTTON_MASK_1, MouseButton::Left),
        (xcb::xproto::BUTTON_MASK_2, MouseButton::Middle),
        (xcb::xproto::BUTTON_MASK_3, MouseButton::Right),
        // TODO: determine the X1/X2 state, using our own caching if necessary.
        // BUTTON_MASK_4/5 do not work: they are for scroll events.
    ];
    for (mask, button) in button_masks {
        if mods & (*mask as u16) != 0 {
            buttons.insert(*button);
        }
    }
    buttons
}

// Extracts the keyboard modifiers from, e.g., the `state` field of
// `xcb::xproto::ButtonPressEvent`
fn key_mods(mods: u16) -> KeyModifiers {
    let mut ret = KeyModifiers::default();
    let mut key_masks = [
        (xcb::xproto::MOD_MASK_SHIFT, &mut ret.shift),
        (xcb::xproto::MOD_MASK_CONTROL, &mut ret.ctrl),
        // X11's mod keys are configurable, but this seems
        // like a reasonable default for US keyboards, at least,
        // where the "windows" key seems to be MOD_MASK_4.
        (xcb::xproto::MOD_MASK_1, &mut ret.alt),
        (xcb::xproto::MOD_MASK_4, &mut ret.meta),
    ];
    for (mask, key) in &mut key_masks {
        if mods & (*mask as u16) != 0 {
            **key = true;
        }
    }
    ret
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
    id: u32,
    window: Weak<Window>,
}

impl WindowHandle {
    fn new(id: u32, window: Weak<Window>) -> WindowHandle {
        WindowHandle { id, window }
    }

    pub fn show(&self) {
        if let Some(w) = self.window.upgrade() {
            w.show();
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn close(&self) {
        if let Some(w) = self.window.upgrade() {
            w.close();
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn resizable(&self, resizable: bool) {
        if let Some(w) = self.window.upgrade() {
            w.resizable(resizable);
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        if let Some(w) = self.window.upgrade() {
            w.show_titlebar(show_titlebar);
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn bring_to_front_and_focus(&self) {
        if let Some(w) = self.window.upgrade() {
            w.bring_to_front_and_focus();
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn invalidate(&self) {
        if let Some(w) = self.window.upgrade() {
            w.invalidate();
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(w) = self.window.upgrade() {
            w.invalidate_rect(rect);
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn set_title(&self, title: &str) {
        if let Some(w) = self.window.upgrade() {
            w.set_title(title);
        } else {
            log::error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn set_menu(&self, menu: Menu) {
        if let Some(w) = self.window.upgrade() {
            w.set_menu(menu);
        } else {
            log::error!("Window {} has already been dropped", self.id);
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
            log::error!("Window {} has already been dropped", self.id);
            96.0
        }
    }
}
