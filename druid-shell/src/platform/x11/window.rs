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

use anyhow::{anyhow, Context, Error};
use cairo::{XCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    self, AtomEnum, ConnectionExt, EventMask, ExposeEvent, PropMode, Visualtype, WindowClass,
};
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

use crate::alert::{AlertOptions, AlertResponse, AlertToken};
use crate::dialog::{FileDialogOptions, FileInfo};
use crate::error::Error as ShellError;
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::keycodes::KeyCode;
use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
use crate::piet::{Piet, RenderContext};
use crate::scale::Scale;
use crate::window::{IdleToken, Text, TimerToken, WinHandler};

use super::application::Application;
use super::menu::Menu;
use super::util;

/// A version of XCB's `xcb_visualtype_t` struct. This was copied from the [example] in x11rb; it
/// is used to interoperate with cairo.
///
/// The official upstream reference for this struct definition is [here].
///
/// [example]: https://github.com/psychon/x11rb/blob/master/cairo-example/src/main.rs
/// [here]: https://xcb.freedesktop.org/manual/structxcb__visualtype__t.html
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct xcb_visualtype_t {
    pub visual_id: u32,
    pub class: u8,
    pub bits_per_rgb_value: u8,
    pub colormap_entries: u16,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub pad0: [u8; 4],
}

impl From<Visualtype> for xcb_visualtype_t {
    fn from(value: Visualtype) -> xcb_visualtype_t {
        xcb_visualtype_t {
            visual_id: value.visual_id,
            class: value.class.into(),
            bits_per_rgb_value: value.bits_per_rgb_value,
            colormap_entries: value.colormap_entries,
            red_mask: value.red_mask,
            green_mask: value.green_mask,
            blue_mask: value.blue_mask,
            pad0: [0; 4],
        }
    }
}

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
        let atoms = WindowAtoms::new(conn.as_ref())
            .context("failed to intern X11 atoms")?
            .reply()
            .context("failed to get back X11 atoms")?;

        // Replace the window's WM_PROTOCOLS with the following.
        let protocols = [atoms.WM_DELETE_WINDOW];
        // TODO(x11/errors): Check the response for errors?
        conn.change_property32(
            PropMode::Replace,
            window_id,
            atoms.WM_PROTOCOLS,
            AtomEnum::ATOM,
            &protocols,
        )?;

        Ok(atoms)
    }

    /// Create a new cairo `Context`.
    fn create_cairo_context(
        &self,
        window_id: u32,
        visual_type: &Visualtype,
    ) -> Result<RefCell<cairo::Context>, Error> {
        let conn = self.app.connection();
        let cairo_xcb_connection = unsafe {
            XCBConnection::from_raw_none(
                conn.get_raw_xcb_connection() as *mut cairo_sys::xcb_connection_t
            )
        };
        let cairo_drawable = XCBDrawable(window_id);
        let mut xcb_visual = xcb_visualtype_t::from(*visual_type);
        let cairo_visual_type = unsafe {
            XCBVisualType::from_raw_none(
                &mut xcb_visual as *mut xcb_visualtype_t as *mut cairo_sys::xcb_visualtype_t,
            )
        };
        let cairo_surface = XCBSurface::create(
            &cairo_xcb_connection,
            &cairo_drawable,
            &cairo_visual_type,
            self.size.width as i32,
            self.size.height as i32,
        )
        .map_err(|status| anyhow!("Failed to create cairo surface: {}", status))?;

        Ok(RefCell::new(cairo::Context::new(&cairo_surface)))
    }

    // TODO(x11/menus): make menus if requested
    pub fn build(self) -> Result<WindowHandle, Error> {
        let conn = self.app.connection();
        let screen_num = self.app.screen_num();
        let id = conn.generate_id()?;
        let setup = conn.setup();
        // TODO(x11/errors): Don't unwrap for screen or visual_type?
        let screen = setup.roots.get(screen_num as usize).unwrap();
        let visual_type = util::get_visual_from_screen(&screen).unwrap();
        let visual_id = visual_type.visual_id;

        let cw_values = xproto::CreateWindowAux::new()
            .background_pixel(screen.white_pixel)
            .event_mask(
                EventMask::Exposure
                    | EventMask::StructureNotify
                    | EventMask::KeyPress
                    | EventMask::KeyRelease
                    | EventMask::ButtonPress
                    | EventMask::ButtonRelease
                    | EventMask::PointerMotion,
            );

        // Create the actual window
        conn.create_window(
            // Window depth
            x11rb::COPY_FROM_PARENT.try_into().unwrap(),
            // The new window's ID
            id,
            // Parent window of this new window
            // TODO(#468): either `screen.root()` (no parent window) or pass parent here to attach
            screen.root,
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
            WindowClass::InputOutput,
            // Visual ID
            visual_id,
            // Window properties mask
            &cw_values,
        )?
        .check()?;

        // TODO(x11/errors): Should do proper cleanup (window destruction etc) in case of error
        let atoms = self.atoms(id)?;
        let cairo_context = self.create_cairo_context(id, &visual_type)?;
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

// This creates a `struct WindowAtoms` containing the specified atoms as members (along with some
// convenience methods to intern and query those atoms). We use the following atoms:
//
// WM_PROTOCOLS
//
// List of atoms that identify the communications protocols between
// the client and window manager in which the client is willing to participate.
//
// https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#wm_protocols_property
//
// WM_DELETE_WINDOW
//
// Including this atom in the WM_PROTOCOLS property on each window makes sure that
// if the window manager respects WM_DELETE_WINDOW it will send us the event.
//
// The WM_DELETE_WINDOW event is sent when there is a request to close the window.
// Registering for but ignoring this event means that the window will remain open.
//
// https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#window_deletion
atom_manager! {
    WindowAtoms: WindowAtomsCookie {
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
    }
}

/// The mutable state of the window.
struct WindowState {
    size: Size,
}

impl Window {
    fn connect(&self, handle: WindowHandle) -> Result<(), Error> {
        let mut handler = borrow_mut!(self.handler)?;
        let size = self.size()?;
        handler.connect(&handle.into());
        handler.scale(Scale::default());
        handler.size(size);
        Ok(())
    }

    /// Start the destruction of the window.
    ///
    /// ### Connection
    ///
    /// Does not flush the connection.
    pub fn destroy(&self) {
        log_x11!(self.app.connection().destroy_window(self.id));
    }

    fn cairo_surface(&self) -> Result<XCBSurface, Error> {
        borrow!(self.cairo_context)?
            .get_target()
            .try_into()
            .map_err(|_| anyhow!("Window::cairo_surface try_into"))
    }

    fn size(&self) -> Result<Size, Error> {
        Ok(borrow!(self.state)?.size)
    }

    fn set_size(&self, size: Size) -> Result<(), Error> {
        // TODO(x11/dpi_scaling): detect DPI and scale size
        let new_size = {
            let mut state = borrow_mut!(self.state)?;
            if size != state.size {
                state.size = size;
                Some(size)
            } else {
                None
            }
        };
        if let Some(size) = new_size {
            self.cairo_surface()?
                .set_size(size.width as i32, size.height as i32)
                .map_err(|status| {
                    anyhow!(
                        "Failed to update cairo surface size to {:?}: {}",
                        size,
                        status
                    )
                })?;
            borrow_mut!(self.handler)?.size(size);
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
        let expose_event = ExposeEvent {
            window: self.id,
            x: rect.x0 as u16,
            y: rect.y0 as u16,
            width: rect.width() as u16,
            height: rect.height() as u16,
            count: 0,
            response_type: x11rb::protocol::xproto::EXPOSE_EVENT,
            sequence: 0,
        };
        log_x11!(self.app.connection().send_event(
            false,
            self.id,
            EventMask::Exposure,
            expose_event,
        ));
    }

    fn render(&self, invalid_rect: Rect) -> Result<(), Error> {
        // Figure out the window's current size
        let reply = self.app.connection().get_geometry(self.id)?.reply()?;
        let size = Size::new(reply.width as f64, reply.height as f64);
        self.set_size(size)?;

        let mut anim = false;
        {
            let mut cairo_ctx = borrow_mut!(self.cairo_context)?;
            let mut piet_ctx = Piet::new(&mut cairo_ctx);
            piet_ctx.clip(invalid_rect);

            // We need to be careful with earlier returns here, because piet_ctx
            // can panic if it isn't finish()ed. Also, we want to reset cairo's clip
            // even on error.
            let err;
            match borrow_mut!(self.handler) {
                Ok(mut handler) => {
                    anim = handler.paint(&mut piet_ctx, invalid_rect);
                    err = piet_ctx
                        .finish()
                        .map_err(|e| anyhow!("Window::render - piet finish failed: {}", e));
                }
                Err(e) => {
                    err = Err(e);
                    if let Err(e) = piet_ctx.finish() {
                        // We can't return both errors, so just log this one.
                        log::error!("Window::render - piet finish failed in error branch: {}", e);
                    }
                }
            };
            cairo_ctx.reset_clip();

            err?;
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
        self.app.connection().flush()?;
        Ok(())
    }

    fn show(&self) {
        log_x11!(self.app.connection().map_window(self.id));
        log_x11!(self.app.connection().flush());
    }

    fn close(&self) {
        self.destroy();
        log_x11!(self.app.connection().flush());
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
        let conn = self.app.connection();
        log_x11!(conn.configure_window(
            self.id,
            &xproto::ConfigureWindowAux::new().stack_mode(xproto::StackMode::Above),
        ));
        log_x11!(conn.set_input_focus(
            xproto::InputFocus::PointerRoot,
            self.id,
            xproto::Time::CurrentTime,
        ));
        log_x11!(conn.flush());
    }

    fn invalidate(&self) {
        match self.size() {
            Ok(size) => self.invalidate_rect(size.to_rect()),
            Err(err) => log::error!("Window::invalidate - failed to get size: {}", err),
        }
    }

    fn invalidate_rect(&self, rect: Rect) {
        self.request_redraw(rect);
        log_x11!(self.app.connection().flush());
    }

    fn set_title(&self, title: &str) {
        log_x11!(self.app.connection().change_property8(
            xproto::PropMode::Replace,
            self.id,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        ));
        log_x11!(self.app.connection().flush());
    }

    fn set_menu(&self, _menu: Menu) {
        // TODO(x11/menus): implement Window::set_menu (currently a no-op)
    }

    fn get_scale(&self) -> Result<Scale, Error> {
        // TODO(x11/dpi_scaling): figure out DPI scaling
        Ok(Scale::new(1.0, 1.0))
    }

    pub fn handle_expose(&self, expose: &xproto::ExposeEvent) -> Result<(), Error> {
        // TODO(x11/dpi_scaling): when dpi scaling is
        // implemented, it needs to be used here too
        let rect = Rect::from_origin_size(
            (expose.x as f64, expose.y as f64),
            (expose.width as f64, expose.height as f64),
        );
        Ok(self.render(rect)?)
    }

    pub fn handle_key_press(&self, key_press: &xproto::KeyPressEvent) -> Result<(), Error> {
        let key: u32 = key_press.detail as u32;
        let key_code: KeyCode = key.into();
        let key_event = KeyEvent::new(
            key_code,
            // TODO: detect repeated keys
            false,
            key_mods(key_press.state),
            key_code,
            key_code,
        );
        borrow_mut!(self.handler)?.key_down(key_event);
        Ok(())
    }

    pub fn handle_button_press(
        &self,
        button_press: &xproto::ButtonPressEvent,
    ) -> Result<(), Error> {
        let button = mouse_button(button_press.detail);
        let mouse_event = MouseEvent {
            pos: Point::new(button_press.event_x as f64, button_press.event_y as f64),
            // The xcb state field doesn't include the newly pressed button, but
            // druid wants it to be included.
            buttons: mouse_buttons(button_press.state).with(button),
            mods: key_mods(button_press.state),
            // TODO: detect the count
            count: 1,
            focus: false,
            button,
            wheel_delta: Vec2::ZERO,
        };
        borrow_mut!(self.handler)?.mouse_down(&mouse_event);
        Ok(())
    }

    pub fn handle_button_release(
        &self,
        button_release: &xproto::ButtonReleaseEvent,
    ) -> Result<(), Error> {
        let button = mouse_button(button_release.detail);
        let mouse_event = MouseEvent {
            pos: Point::new(button_release.event_x as f64, button_release.event_y as f64),
            // The xcb state includes the newly released button, but druid
            // doesn't want it.
            buttons: mouse_buttons(button_release.state).without(button),
            mods: key_mods(button_release.state),
            count: 0,
            focus: false,
            button,
            wheel_delta: Vec2::ZERO,
        };
        borrow_mut!(self.handler)?.mouse_up(&mouse_event);
        Ok(())
    }

    pub fn handle_wheel(&self, event: &xproto::ButtonPressEvent) -> Result<(), Error> {
        let button = event.detail;
        let mods = key_mods(event.state);

        // We use a delta of 120 per tick to match the behavior of Windows.
        let delta = match button {
            4 if mods.shift => (-120.0, 0.0),
            4 => (0.0, -120.0),
            5 if mods.shift => (120.0, 0.0),
            5 => (0.0, 120.0),
            6 => (-120.0, 0.0),
            7 => (120.0, 0.0),
            _ => return Err(anyhow!("unexpected mouse wheel button: {}", button)),
        };
        let mouse_event = MouseEvent {
            pos: Point::new(event.event_x as f64, event.event_y as f64),
            buttons: mouse_buttons(event.state),
            mods: key_mods(event.state),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: delta.into(),
        };

        borrow_mut!(self.handler)?.wheel(&mouse_event);
        Ok(())
    }

    pub fn handle_motion_notify(
        &self,
        motion_notify: &xproto::MotionNotifyEvent,
    ) -> Result<(), Error> {
        let mouse_event = MouseEvent {
            pos: Point::new(motion_notify.event_x as f64, motion_notify.event_y as f64),
            buttons: mouse_buttons(motion_notify.state),
            mods: key_mods(motion_notify.state),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        };
        borrow_mut!(self.handler)?.mouse_move(&mouse_event);
        Ok(())
    }

    pub fn handle_client_message(
        &self,
        client_message: &xproto::ClientMessageEvent,
    ) -> Result<(), Error> {
        // https://www.x.org/releases/X11R7.7/doc/libX11/libX11/libX11.html#id2745388
        // https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#window_deletion
        if client_message.type_ == self.atoms.WM_PROTOCOLS && client_message.format == 32 {
            let protocol = client_message.data.as_data32()[0];
            if protocol == self.atoms.WM_DELETE_WINDOW {
                self.close();
            }
        }
        Ok(())
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn handle_destroy_notify(
        &self,
        _destroy_notify: &xproto::DestroyNotifyEvent,
    ) -> Result<(), Error> {
        borrow_mut!(self.handler)?.destroy();
        Ok(())
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
        (xproto::ButtonMask::M1, MouseButton::Left),
        (xproto::ButtonMask::M2, MouseButton::Middle),
        (xproto::ButtonMask::M3, MouseButton::Right),
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
        (xproto::ModMask::Shift, &mut ret.shift),
        (xproto::ModMask::Control, &mut ret.ctrl),
        // X11's mod keys are configurable, but this seems
        // like a reasonable default for US keyboards, at least,
        // where the "windows" key seems to be MOD_MASK_4.
        (xproto::ModMask::M1, &mut ret.alt),
        (xproto::ModMask::M4, &mut ret.meta),
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

    /// Show an alert dialog.
    pub fn alert(&self, _options: AlertOptions) -> AlertToken {
        {
            // Just shutting up clippy until the real implementation arrives
            AlertResponse::new(AlertToken::new(1), None);
        }
        AlertToken::INVALID
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

    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        if let Some(w) = self.window.upgrade() {
            Ok(w.get_scale()?)
        } else {
            log::error!("Window {} has already been dropped", self.id);
            Ok(Scale::new(1.0, 1.0))
        }
    }
}
