// Copyright 2020 The Druid Authors.
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

use std::cell::{Cell, RefCell};
use std::collections::BinaryHeap;
use std::convert::{TryFrom, TryInto};
use std::os::unix::io::RawFd;
use std::panic::Location;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::scale::Scalable;
use anyhow::{anyhow, Context, Error};
use cairo::{XCBConnection as CairoXCBConnection, XCBDrawable, XCBSurface, XCBVisualType};
use tracing::{error, info, warn};
use x11rb::connection::Connection;
use x11rb::errors::ReplyOrIdError;
use x11rb::properties::{WmHints, WmHintsState, WmSizeHints};
use x11rb::protocol::present::{CompleteNotifyEvent, ConnectionExt as _, IdleNotifyEvent};
use x11rb::protocol::render::{ConnectionExt as _, Pictformat};
use x11rb::protocol::xfixes::{ConnectionExt as _, Region as XRegion};
use x11rb::protocol::xproto::{
    self, AtomEnum, ChangeWindowAttributesAux, ColormapAlloc, ConfigureNotifyEvent,
    ConfigureWindowAux, ConnectionExt, CreateGCAux, EventMask, Gcontext, ImageFormat,
    ImageOrder as X11ImageOrder, Pixmap, PropMode, Rectangle, Visualtype, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;
use x11rb::xcb_ffi::XCBConnection;

#[cfg(feature = "raw-win-handle")]
use raw_window_handle::{unix::XcbHandle, HasRawWindowHandle, RawWindowHandle};

use crate::common_util::IdleCallback;
use crate::dialog::FileDialogOptions;
use crate::error::Error as ShellError;
use crate::keyboard::{KeyState, Modifiers};
use crate::kurbo::{Insets, Point, Rect, Size, Vec2};
use crate::mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent};
use crate::piet::{Piet, PietText, RenderContext};
use crate::region::Region;
use crate::scale::Scale;
use crate::text::{simulate_input, Event};
use crate::window::{
    FileDialogToken, IdleToken, TextFieldToken, TimerToken, WinHandler, WindowLevel,
};
use crate::{window, KeyEvent, ScaledArea};

use super::application::Application;
use super::menu::Menu;
use super::util::Timer;

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

fn size_hints(resizable: bool, size: Size, min_size: Size) -> WmSizeHints {
    let mut size_hints = WmSizeHints::new();
    if resizable {
        size_hints.min_size = Some((min_size.width as i32, min_size.height as i32));
    } else {
        size_hints.min_size = Some((size.width as i32, size.height as i32));
        size_hints.max_size = Some((size.width as i32, size.height as i32));
    }
    size_hints
}

pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    transparent: bool,
    position: Option<Point>,
    size: Size,
    min_size: Size,
    resizable: bool,
    level: WindowLevel,
    state: Option<window::WindowState>,
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
            handler: None,
            title: String::new(),
            transparent: false,
            position: None,
            size: Size::new(500.0, 400.0),
            min_size: Size::new(0.0, 0.0),
            resizable: true,
            level: WindowLevel::AppWindow,
            state: None,
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        // zero sized window results in server error
        self.size = if size.width == 0. || size.height == 0. {
            Size::new(1., 1.)
        } else {
            size
        };
    }

    pub fn set_min_size(&mut self, min_size: Size) {
        self.min_size = min_size;
    }

    pub fn resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    pub fn show_titlebar(&mut self, _show_titlebar: bool) {
        // not sure how to do this, maybe _MOTIF_WM_HINTS?
        warn!("WindowBuilder::show_titlebar is currently unimplemented for X11 backend.");
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = Some(position);
    }

    pub fn set_level(&mut self, level: window::WindowLevel) {
        self.level = level;
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        self.state = Some(state);
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, _menu: Menu) {
        // TODO(x11/menus): implement WindowBuilder::set_menu (currently a no-op)
    }

    fn create_cairo_surface(
        &self,
        window_id: u32,
        visual_type: &Visualtype,
    ) -> Result<XCBSurface, Error> {
        let conn = self.app.connection();
        let cairo_xcb_connection = unsafe {
            CairoXCBConnection::from_raw_none(
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
        Ok(cairo_surface)
    }

    // TODO(x11/menus): make menus if requested
    pub fn build(self) -> Result<WindowHandle, Error> {
        let conn = self.app.connection();
        let screen_num = self.app.screen_num();
        let id = conn.generate_id()?;
        let setup = conn.setup();

        let env_dpi = std::env::var("DRUID_X11_DPI")
            .ok()
            .map(|x| x.parse::<f64>());

        let scale = match env_dpi.or_else(|| self.app.rdb.get_value("Xft.dpi", "").transpose()) {
            Some(Ok(dpi)) => {
                let scale = dpi / 96.;
                Scale::new(scale, scale)
            }
            None => Scale::default(),
            Some(Err(err)) => {
                let default = Scale::default();
                warn!(
                    "Unable to parse dpi: {:?}, defaulting to {:?}",
                    err, default
                );
                default
            }
        };

        let size_px = self.size.to_px(scale);
        let screen = setup
            .roots
            .get(screen_num)
            .ok_or_else(|| anyhow!("Invalid screen num: {}", screen_num))?;
        let visual_type = if self.transparent {
            self.app.argb_visual_type()
        } else {
            None
        };
        let (transparent, visual_type) = match visual_type {
            Some(visual) => (true, visual),
            None => (false, self.app.root_visual_type()),
        };
        if transparent != self.transparent {
            warn!("Windows with transparent backgrounds do not work");
        }

        let mut cw_values = xproto::CreateWindowAux::new().event_mask(
            EventMask::EXPOSURE
                | EventMask::STRUCTURE_NOTIFY
                | EventMask::KEY_PRESS
                | EventMask::KEY_RELEASE
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::POINTER_MOTION,
        );
        if transparent {
            let colormap = conn.generate_id()?;
            conn.create_colormap(
                ColormapAlloc::NONE,
                colormap,
                screen.root,
                visual_type.visual_id,
            )?;
            cw_values = cw_values
                .border_pixel(screen.white_pixel)
                .colormap(colormap);
        };

        let (parent, parent_origin) = match &self.level {
            WindowLevel::AppWindow => (Weak::new(), Vec2::ZERO),
            WindowLevel::Tooltip(parent)
            | WindowLevel::DropDown(parent)
            | WindowLevel::Modal(parent) => {
                let handle = parent.0.window.clone();
                let origin = handle
                    .upgrade()
                    .map(|x| x.get_position())
                    .unwrap_or_default()
                    .to_vec2();
                (handle, origin)
            }
        };
        let pos = (self.position.unwrap_or_default() + parent_origin).to_px(scale);

        // Create the actual window
        let (width_px, height_px) = (size_px.width as u16, size_px.height as u16);
        let depth = if transparent { 32 } else { screen.root_depth };
        conn.create_window(
            // Window depth
            depth,
            // The new window's ID
            id,
            // Parent window of this new window
            // TODO(#468): either `screen.root()` (no parent window) or pass parent here to attach
            screen.root,
            // X-coordinate of the new window
            pos.x as _,
            // Y-coordinate of the new window
            pos.y as _,
            // Width of the new window
            width_px,
            // Height of the new window
            height_px,
            // Border width
            0,
            // Window class type
            WindowClass::INPUT_OUTPUT,
            // Visual ID
            visual_type.visual_id,
            // Window properties mask
            &cw_values,
        )?
        .check()
        .context("create window")?;

        if let Some(colormap) = cw_values.colormap {
            conn.free_colormap(colormap)?;
        }

        // Allocate a graphics context (currently used only for copying pixels when present is
        // unavailable).
        let gc = conn.generate_id()?;
        conn.create_gc(gc, id, &CreateGCAux::new())?
            .check()
            .context("create graphics context")?;

        // TODO(x11/errors): Should do proper cleanup (window destruction etc) in case of error
        let cairo_surface = RefCell::new(self.create_cairo_surface(id, &visual_type)?);
        let present_data = match self.initialize_present_data(id) {
            Ok(p) => Some(p),
            Err(e) => {
                info!("Failed to initialize present extension: {}", e);
                None
            }
        };
        let handler = RefCell::new(self.handler.unwrap());
        // When using present, we generally need two buffers (because after we present, we aren't
        // allowed to use that buffer for a little while, and so we might want to render to the
        // other one). Otherwise, we only need one.
        let buf_count = if present_data.is_some() { 2 } else { 1 };
        let buffers = RefCell::new(Buffers::new(
            conn, id, buf_count, width_px, height_px, depth,
        )?);

        // Initialize some properties
        let atoms = self.app.atoms();
        let pid = nix::unistd::Pid::this().as_raw();
        if let Ok(pid) = u32::try_from(pid) {
            conn.change_property32(
                xproto::PropMode::REPLACE,
                id,
                atoms._NET_WM_PID,
                AtomEnum::CARDINAL,
                &[pid],
            )?
            .check()
            .context("set _NET_WM_PID")?;
        }

        if let Some(name) = std::env::args_os().next() {
            // ICCCM § 4.1.2.5:
            // The WM_CLASS property (of type STRING without control characters) contains two
            // consecutive null-terminated strings. These specify the Instance and Class names.
            //
            // The code below just imitates what happens on the gtk backend:
            // - instance: The program's name
            // - class: The program's name with first letter in upper case

            // Get the name of the running binary
            let path: &std::path::Path = name.as_ref();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Build the contents of WM_CLASS
            let mut wm_class = Vec::with_capacity(2 * (name.len() + 1));
            wm_class.extend(name.as_bytes());
            wm_class.push(0);
            if let Some(&first) = wm_class.get(0) {
                wm_class.push(first.to_ascii_uppercase());
                wm_class.extend(&name.as_bytes()[1..]);
            }
            wm_class.push(0);
            conn.change_property8(
                PropMode::REPLACE,
                id,
                AtomEnum::WM_CLASS,
                AtomEnum::STRING,
                &wm_class,
            )?;
        } else {
            // GTK (actually glib) goes fishing in /proc (platform_get_argv0()). We pass.
        }

        // Replace the window's WM_PROTOCOLS with the following.
        let protocols = [atoms.WM_DELETE_WINDOW];
        conn.change_property32(
            PropMode::REPLACE,
            id,
            atoms.WM_PROTOCOLS,
            AtomEnum::ATOM,
            &protocols,
        )?
        .check()
        .context("set WM_PROTOCOLS")?;

        let min_size = self.min_size.to_px(scale);
        log_x11!(size_hints(self.resizable, size_px, min_size)
            .set_normal_hints(conn.as_ref(), id)
            .context("set wm normal hints"));

        // TODO: set _NET_WM_STATE
        let mut hints = WmHints::new();
        if let Some(state) = self.state {
            hints.initial_state = Some(match state {
                window::WindowState::Maximized => WmHintsState::Normal,
                window::WindowState::Minimized => WmHintsState::Iconic,
                window::WindowState::Restored => WmHintsState::Normal,
            });
        }
        log_x11!(hints.set(conn.as_ref(), id).context("set wm hints"));

        // set level
        {
            let window_type = match self.level {
                WindowLevel::AppWindow => atoms._NET_WM_WINDOW_TYPE_NORMAL,
                WindowLevel::Tooltip(_) => atoms._NET_WM_WINDOW_TYPE_TOOLTIP,
                WindowLevel::Modal(_) => atoms._NET_WM_WINDOW_TYPE_DIALOG,
                WindowLevel::DropDown(_) => atoms._NET_WM_WINDOW_TYPE_DROPDOWN_MENU,
            };

            let conn = self.app.connection();
            log_x11!(conn.change_property32(
                xproto::PropMode::REPLACE,
                id,
                atoms._NET_WM_WINDOW_TYPE,
                AtomEnum::ATOM,
                &[window_type],
            ));
            if matches!(
                self.level,
                WindowLevel::DropDown(_) | WindowLevel::Modal(_) | WindowLevel::Tooltip(_)
            ) {
                log_x11!(conn.change_window_attributes(
                    id,
                    &ChangeWindowAttributesAux::new().override_redirect(1),
                ));
            }
        }

        let window = Rc::new(Window {
            id,
            gc,
            app: self.app.clone(),
            handler,
            cairo_surface,
            area: Cell::new(ScaledArea::from_px(size_px, scale)),
            scale: Cell::new(scale),
            min_size,
            invalid: RefCell::new(Region::EMPTY),
            destroyed: Cell::new(false),
            timer_queue: Mutex::new(BinaryHeap::new()),
            idle_queue: Arc::new(Mutex::new(Vec::new())),
            idle_pipe: self.app.idle_pipe(),
            present_data: RefCell::new(present_data),
            buffers,
            active_text_field: Cell::new(None),
            parent,
        });

        window.set_title(&self.title);
        if let Some(pos) = self.position {
            window.set_position(pos);
        }

        let handle = WindowHandle::new(id, Rc::downgrade(&window));
        window.connect(handle.clone())?;

        self.app.add_window(id, window)?;

        Ok(handle)
    }

    fn initialize_present_data(&self, window_id: u32) -> Result<PresentData, Error> {
        if self.app.present_opcode().is_some() {
            let conn = self.app.connection();

            // We use the CompleteNotify events to schedule the next frame, and the IdleNotify
            // events to manage our buffers.
            let id = conn.generate_id()?;
            use x11rb::protocol::present::EventMask;
            conn.present_select_input(
                id,
                window_id,
                EventMask::COMPLETE_NOTIFY | EventMask::IDLE_NOTIFY,
            )?
            .check()
            .context("set present event mask")?;

            let region_id = conn.generate_id()?;
            conn.xfixes_create_region(region_id, &[])
                .context("create region")?;

            Ok(PresentData {
                serial: 0,
                region: region_id,
                waiting_on: None,
                needs_present: false,
                last_msc: None,
                last_ust: None,
            })
        } else {
            Err(anyhow!("no present opcode"))
        }
    }
}

/// An X11 window.
//
// We use lots of RefCells here, so to avoid panics we need some rules. The basic observation is
// that there are two ways we can end up calling the code in this file:
//
// 1) it either comes from the system (e.g. through some X11 event), or
// 2) from the client (e.g. druid, calling a method on its `WindowHandle`).
//
// Note that 2 only ever happens as a result of 1 (i.e., the system calls us, we call the client
// using the `WinHandler`, and it calls us back). The rules are:
//
// a) We never call into the system as a result of 2. As a consequence, we never get 1
//    re-entrantly.
// b) We *almost* never call into the `WinHandler` while holding any of the other RefCells. There's
//    an exception for `paint`. This is enforced by the `with_handler` method.
//    (TODO: we could try to encode this exception statically, by making the data accessible in
//    case 2 smaller than the data accessible in case 1).
pub(crate) struct Window {
    id: u32,
    gc: Gcontext,
    app: Application,
    handler: RefCell<Box<dyn WinHandler>>,
    cairo_surface: RefCell<XCBSurface>,
    area: Cell<ScaledArea>,
    scale: Cell<Scale>,
    // min size in px
    min_size: Size,
    /// We've told X11 to destroy this window, so don't so any more X requests with this window id.
    destroyed: Cell<bool>,
    /// The region that was invalidated since the last time we rendered.
    invalid: RefCell<Region>,
    /// Timers, sorted by "earliest deadline first"
    timer_queue: Mutex<BinaryHeap<Timer>>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    // Writing to this wakes up the event loop, so that it can run idle handlers.
    idle_pipe: RawFd,

    /// When this is `Some(_)`, we use the X11 Present extension to present windows. This syncs all
    /// presentation to vblank and it appears to prevent tearing (subject to various caveats
    /// regarding broken video drivers).
    ///
    /// The Present extension works roughly like this: we submit a pixmap for presentation. It will
    /// get drawn at the next vblank, and some time shortly after that we'll get a notification
    /// that the drawing was completed.
    ///
    /// There are three ways that rendering can get triggered:
    /// 1) We render a frame, and it signals to us that an animation is requested. In this case, we
    ///     will render the next frame as soon as we get a notification that the just-presented
    ///     frame completed. In other words, we use `CompleteNotifyEvent` to schedule rendering.
    /// 2) We get an expose event telling us that a region got invalidated. In
    ///    this case, we will render the next frame immediately unless we're already waiting for a
    ///    completion notification. (If we are waiting for a completion notification, we just make
    ///    a note to schedule a new frame once we get it.)
    /// 3) Someone calls `invalidate` or `invalidate_rect` on us. We schedule ourselves to repaint
    ///    in the idle loop. This is better than rendering straight away, because for example they
    ///    might have called `invalidate` from their paint callback, and then we'd end up painting
    ///    re-entrantively.
    ///
    /// This is probably not the best (or at least, not the lowest-latency) scheme we can come up
    /// with, because invalidations that happen shortly after a vblank might need to wait 2 frames
    /// before they appear. If we're getting lots of invalidations, it might be better to render more
    /// than once per frame. Note that if we do, it will require some changes to part 1) above,
    /// because if we render twice in a frame then we will get two completion notifications in a
    /// row, so we don't want to present on both of them. The `msc` field of the completion
    /// notification might be useful here, because it allows us to check how many frames have
    /// actually been presented.
    present_data: RefCell<Option<PresentData>>,
    buffers: RefCell<Buffers>,
    active_text_field: Cell<Option<TextFieldToken>>,
    parent: Weak<Window>,
}

/// A collection of pixmaps for rendering to. This gets used in two different ways: if the present
/// extension is enabled, we render to a pixmap and then present it. If the present extension is
/// disabled, we render to a pixmap and then call `copy_area` on it (this probably isn't the best
/// way to imitate double buffering, but it's the fallback anyway).
struct Buffers {
    /// A list of idle pixmaps. We take a pixmap from here for rendering to.
    ///
    /// When we're not using the present extension, all pixmaps belong in here; as soon as we copy
    /// from one, we can use it again.
    ///
    /// When we submit a pixmap to present, we're not allowed to touch it again until we get a
    /// corresponding IDLE_NOTIFY event. In my limited experiments this happens shortly after
    /// vsync, meaning that we may want to start rendering the next pixmap before we get the old
    /// one back. Therefore, we keep a list of pixmaps. We pop one each time we render, and push
    /// one when we get IDLE_NOTIFY.
    ///
    /// Since the current code only renders at most once per vsync, two pixmaps seems to always be
    /// enough. Nevertheless, we will allocate more on the fly if we need them. Note that rendering
    /// more than once per vsync can only improve latency, because only the most recently-presented
    /// pixmap will get rendered.
    idle_pixmaps: Vec<Pixmap>,
    /// A list of all the allocated pixmaps (including the idle ones).
    all_pixmaps: Vec<Pixmap>,
    /// The sizes of the pixmaps (they all have the same size). In order to avoid repeatedly
    /// reallocating as the window size changes, we allow these to be bigger than the window.
    width: u16,
    height: u16,
    /// The depth of the currently allocated pixmaps.
    depth: u8,
}

/// The state involved in using X's [Present] extension.
///
/// [Present]: https://cgit.freedesktop.org/xorg/proto/presentproto/tree/presentproto.txt
#[derive(Debug)]
struct PresentData {
    /// A monotonically increasing present request counter.
    serial: u32,
    /// The region that we use for telling X what to present.
    region: XRegion,
    /// Did we submit a present that hasn't completed yet? If so, this is its serial number.
    waiting_on: Option<u32>,
    /// We need to render another frame as soon as the current one is done presenting.
    needs_present: bool,
    /// The last MSC (media stream counter) that was completed. This can be used to diagnose
    /// latency problems, because MSC is a frame counter: it increments once per frame. We should
    /// be presenting on every frame, and storing the last completed MSC lets us know if we missed
    /// one.
    last_msc: Option<u64>,
    /// The time at which the last frame was completed. The present protocol documentation doesn't
    /// define the units, but it appears to be in microseconds.
    last_ust: Option<u64>,
}

#[derive(Clone, PartialEq)]
pub struct CustomCursor(xproto::Cursor);

impl Window {
    #[track_caller]
    fn with_handler<T, F: FnOnce(&mut dyn WinHandler) -> T>(&self, f: F) -> Option<T> {
        if self.cairo_surface.try_borrow_mut().is_err()
            || self.invalid.try_borrow_mut().is_err()
            || self.present_data.try_borrow_mut().is_err()
            || self.buffers.try_borrow_mut().is_err()
        {
            error!("other RefCells were borrowed when calling into the handler");
            return None;
        }

        self.with_handler_and_dont_check_the_other_borrows(f)
    }

    #[track_caller]
    fn with_handler_and_dont_check_the_other_borrows<T, F: FnOnce(&mut dyn WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        match self.handler.try_borrow_mut() {
            Ok(mut h) => Some(f(&mut **h)),
            Err(_) => {
                error!("failed to borrow WinHandler at {}", Location::caller());
                None
            }
        }
    }

    fn connect(&self, handle: WindowHandle) -> Result<(), Error> {
        let size = self.size().size_dp();
        let scale = self.scale.get();
        self.with_handler(|h| {
            h.connect(&handle.into());
            h.scale(scale);
            h.size(size);
        });
        Ok(())
    }

    /// Start the destruction of the window.
    pub fn destroy(&self) {
        if !self.destroyed() {
            self.destroyed.set(true);
            log_x11!(self.app.connection().destroy_window(self.id));
        }
    }

    fn destroyed(&self) -> bool {
        self.destroyed.get()
    }

    fn size(&self) -> ScaledArea {
        self.area.get()
    }

    // note: size is in px
    fn size_changed(&self, size: Size) -> Result<(), Error> {
        let scale = self.scale.get();
        let new_size = {
            if size != self.area.get().size_px() {
                self.area.set(ScaledArea::from_px(size, scale));
                true
            } else {
                false
            }
        };
        if new_size {
            borrow_mut!(self.buffers)?.set_size(
                self.app.connection(),
                self.id,
                size.width as u16,
                size.height as u16,
            );
            borrow_mut!(self.cairo_surface)?
                .set_size(size.width as i32, size.height as i32)
                .map_err(|status| {
                    anyhow!(
                        "Failed to update cairo surface size to {:?}: {}",
                        size,
                        status
                    )
                })?;
            self.add_invalid_rect(size.to_dp(scale).to_rect())?;
            self.with_handler(|h| h.size(size.to_dp(scale)));
            self.with_handler(|h| h.scale(scale));
        }
        Ok(())
    }

    // Ensure that our cairo context is targeting the right drawable, allocating one if necessary.
    fn update_cairo_surface(&self) -> Result<(), Error> {
        let mut buffers = borrow_mut!(self.buffers)?;
        let pixmap = if let Some(p) = buffers.idle_pixmaps.last() {
            *p
        } else {
            info!("ran out of idle pixmaps, creating a new one");
            buffers.create_pixmap(self.app.connection(), self.id)?
        };

        let drawable = XCBDrawable(pixmap);
        borrow_mut!(self.cairo_surface)?
            .set_drawable(&drawable, buffers.width as i32, buffers.height as i32)
            .map_err(|e| anyhow!("Failed to update cairo drawable: {}", e))?;
        Ok(())
    }

    fn render(&self) -> Result<(), Error> {
        self.with_handler(|h| h.prepare_paint());

        if self.destroyed() {
            return Ok(());
        }

        self.update_cairo_surface()?;
        let invalid = std::mem::replace(&mut *borrow_mut!(self.invalid)?, Region::EMPTY);
        {
            let surface = borrow!(self.cairo_surface)?;
            let cairo_ctx = cairo::Context::new(&surface).unwrap();
            let scale = self.scale.get();
            for rect in invalid.rects() {
                let rect = rect.to_px(scale).round();
                cairo_ctx.rectangle(rect.x0, rect.y0, rect.width(), rect.height());
            }
            cairo_ctx.clip();
            cairo_ctx.scale(scale.x(), scale.y());
            let mut piet_ctx = Piet::new(&cairo_ctx);

            // We need to be careful with earlier returns here, because piet_ctx
            // can panic if it isn't finish()ed. Also, we want to reset cairo's clip
            // even on error.
            //
            // Note that we're borrowing the surface while calling the handler. This is ok, because
            // we don't return control to the system or re-borrow the surface from any code that
            // the client can call.
            let result = self.with_handler_and_dont_check_the_other_borrows(|handler| {
                handler.paint(&mut piet_ctx, &invalid);
                piet_ctx
                    .finish()
                    .map_err(|e| anyhow!("Window::render - piet finish failed: {}", e))
            });
            let err = match result {
                None => {
                    // The handler borrow failed, so finish didn't get called.
                    piet_ctx
                        .finish()
                        .map_err(|e| anyhow!("Window::render - piet finish failed: {}", e))
                }
                Some(e) => {
                    // Finish might have errored, in which case we want to propagate it.
                    e
                }
            };
            cairo_ctx.reset_clip();

            err?;
        }

        self.set_needs_present(false)?;

        let mut buffers = borrow_mut!(self.buffers)?;
        let pixmap = *buffers
            .idle_pixmaps
            .last()
            .ok_or_else(|| anyhow!("after rendering, no pixmap to present"))?;
        let scale = self.scale.get();
        if let Some(present) = borrow_mut!(self.present_data)?.as_mut() {
            present.present(self.app.connection(), pixmap, self.id, &invalid, scale)?;
            buffers.idle_pixmaps.pop();
        } else {
            for rect in invalid.rects() {
                let rect = rect.to_px(scale).round();
                let (x, y) = (rect.x0 as i16, rect.y0 as i16);
                let (w, h) = (rect.width() as u16, rect.height() as u16);
                self.app
                    .connection()
                    .copy_area(pixmap, self.id, self.gc, x, y, x, y, w, h)?;
            }
        }
        Ok(())
    }

    fn show(&self) {
        if !self.destroyed() {
            log_x11!(self.app.connection().map_window(self.id));
        }
    }

    fn close(&self) {
        self.destroy();
    }

    /// Set whether the window should be resizable
    fn resizable(&self, resizable: bool) {
        let conn = self.app.connection().as_ref();
        log_x11!(size_hints(resizable, self.size().size_px(), self.min_size)
            .set_normal_hints(conn, self.id)
            .context("set normal hints"));
    }

    /// Set whether the window should show titlebar
    fn show_titlebar(&self, _show_titlebar: bool) {
        warn!("Window::show_titlebar is currently unimplemented for X11 backend.");
    }

    fn parent_origin(&self) -> Vec2 {
        self.parent
            .upgrade()
            .map(|x| x.get_position())
            .unwrap_or_default()
            .to_vec2()
    }

    fn get_position(&self) -> Point {
        fn _get_position(window: &Window) -> Result<Point, Error> {
            let conn = window.app.connection();
            let scale = window.scale.get();
            let geom = conn.get_geometry(window.id)?.reply()?;
            let cord = conn
                .translate_coordinates(window.id, geom.root, 0, 0)?
                .reply()?;
            Ok(Point::new(cord.dst_x as _, cord.dst_y as _).to_dp(scale))
        }
        let pos = _get_position(self);
        log_x11!(&pos);
        pos.map(|pos| pos - self.parent_origin())
            .unwrap_or_default()
    }

    fn set_position(&self, pos: Point) {
        let conn = self.app.connection();
        let scale = self.scale.get();
        let pos = (pos + self.parent_origin()).to_px(scale).expand();
        log_x11!(conn.configure_window(
            self.id,
            &ConfigureWindowAux::new().x(pos.x as i32).y(pos.y as i32),
        ));
    }

    fn set_size(&self, size: Size) {
        let conn = self.app.connection();
        let scale = self.scale.get();
        let size = size.to_px(scale).expand();
        log_x11!(conn.configure_window(
            self.id,
            &ConfigureWindowAux::new()
                .width(size.width as u32)
                .height(size.height as u32),
        ));
    }

    /// Bring this window to the front of the window stack and give it focus.
    fn bring_to_front_and_focus(&self) {
        if self.destroyed() {
            return;
        }

        // TODO(x11/misc): Unsure if this does exactly what the doc comment says; need a test case.
        let conn = self.app.connection();
        log_x11!(conn.configure_window(
            self.id,
            &xproto::ConfigureWindowAux::new().stack_mode(xproto::StackMode::ABOVE),
        ));
        log_x11!(conn.set_input_focus(
            xproto::InputFocus::POINTER_ROOT,
            self.id,
            xproto::Time::CURRENT_TIME,
        ));
    }

    fn add_invalid_rect(&self, rect: Rect) -> Result<(), Error> {
        let scale = self.scale.get();
        borrow_mut!(self.invalid)?.add_rect(rect.to_px(scale).expand().to_dp(scale));
        Ok(())
    }

    /// Redraw more-or-less now.
    ///
    /// "More-or-less" because if we're already waiting on a present, we defer the drawing until it
    /// completes.
    fn redraw_now(&self) -> Result<(), Error> {
        if self.waiting_on_present()? {
            self.set_needs_present(true)?;
        } else {
            self.render()?;
        }
        Ok(())
    }

    /// Schedule a redraw on the idle loop, or if we are waiting on present then schedule it for
    /// when the current present finishes.
    fn request_anim_frame(&self) {
        if let Ok(true) = self.waiting_on_present() {
            if let Err(e) = self.set_needs_present(true) {
                error!(
                    "Window::request_anim_frame - failed to schedule present: {}",
                    e
                );
            }
        } else {
            let idle = IdleHandle {
                queue: Arc::clone(&self.idle_queue),
                pipe: self.idle_pipe,
            };
            idle.schedule_redraw();
        }
    }

    fn invalidate(&self) {
        let rect = self.size().size_dp().to_rect();
        self.add_invalid_rect(rect)
            .unwrap_or_else(|err| error!("Window::invalidate - failed to invalidate: {}", err));

        self.request_anim_frame();
    }

    fn invalidate_rect(&self, rect: Rect) {
        if let Err(err) = self.add_invalid_rect(rect) {
            error!("Window::invalidate_rect - failed to enlarge rect: {}", err);
        }

        self.request_anim_frame();
    }

    fn set_title(&self, title: &str) {
        if self.destroyed() {
            return;
        }

        let atoms = self.app.atoms();

        // This is technically incorrect. STRING encoding is *not* UTF8. However, I am not sure
        // what it really is. WM_LOCALE_NAME might be involved. Hopefully, nothing cares about this
        // as long as _NET_WM_NAME is also set (which uses UTF8).
        log_x11!(self.app.connection().change_property8(
            xproto::PropMode::REPLACE,
            self.id,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            title.as_bytes(),
        ));
        log_x11!(self.app.connection().change_property8(
            xproto::PropMode::REPLACE,
            self.id,
            atoms._NET_WM_NAME,
            atoms.UTF8_STRING,
            title.as_bytes(),
        ));
    }

    fn set_cursor(&self, cursor: &Cursor) {
        let cursors = &self.app.cursors;
        #[allow(deprecated)]
        let cursor = match cursor {
            Cursor::Arrow => cursors.default,
            Cursor::IBeam => cursors.text,
            Cursor::Pointer => cursors.pointer,
            Cursor::Crosshair => cursors.crosshair,
            Cursor::OpenHand => {
                warn!("Cursor::OpenHand not supported for x11 backend. using arrow cursor");
                None
            }
            Cursor::NotAllowed => cursors.not_allowed,
            Cursor::ResizeLeftRight => cursors.col_resize,
            Cursor::ResizeUpDown => cursors.row_resize,
            Cursor::Custom(custom) => Some(custom.0),
        };
        if cursor.is_none() {
            warn!("Unable to load cursor {:?}", cursor);
            return;
        }
        let conn = self.app.connection();
        let changes = ChangeWindowAttributesAux::new().cursor(cursor);
        if let Err(e) = conn.change_window_attributes(self.id, &changes) {
            error!("Changing cursor window attribute failed {}", e);
        };
    }

    fn set_menu(&self, _menu: Menu) {
        // TODO(x11/menus): implement Window::set_menu (currently a no-op)
    }

    fn get_scale(&self) -> Result<Scale, Error> {
        Ok(self.scale.get())
    }

    pub fn handle_expose(&self, expose: &xproto::ExposeEvent) -> Result<(), Error> {
        let rect = Rect::from_origin_size(
            (expose.x as f64, expose.y as f64),
            (expose.width as f64, expose.height as f64),
        )
        .to_dp(self.scale.get());

        self.add_invalid_rect(rect)?;
        if self.waiting_on_present()? {
            self.set_needs_present(true)?;
        } else if expose.count == 0 {
            self.request_anim_frame();
        }
        Ok(())
    }

    pub fn handle_key_event(&self, event: KeyEvent) {
        self.with_handler(|h| match event.state {
            KeyState::Down => {
                simulate_input(h, self.active_text_field.get(), event);
            }
            KeyState::Up => h.key_up(event),
        });
    }

    pub fn handle_button_press(
        &self,
        button_press: &xproto::ButtonPressEvent,
    ) -> Result<(), Error> {
        let button = mouse_button(button_press.detail);
        let scale = self.scale.get();
        let mouse_event = MouseEvent {
            pos: Point::new(button_press.event_x as f64, button_press.event_y as f64).to_dp(scale),
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
        self.with_handler(|h| h.mouse_down(&mouse_event));
        Ok(())
    }

    pub fn handle_button_release(
        &self,
        button_release: &xproto::ButtonReleaseEvent,
    ) -> Result<(), Error> {
        let scale = self.scale.get();
        let button = mouse_button(button_release.detail);
        let mouse_event = MouseEvent {
            pos: Point::new(button_release.event_x as f64, button_release.event_y as f64)
                .to_dp(scale),
            // The xcb state includes the newly released button, but druid
            // doesn't want it.
            buttons: mouse_buttons(button_release.state).without(button),
            mods: key_mods(button_release.state),
            count: 0,
            focus: false,
            button,
            wheel_delta: Vec2::ZERO,
        };
        self.with_handler(|h| h.mouse_up(&mouse_event));
        Ok(())
    }

    pub fn handle_wheel(&self, event: &xproto::ButtonPressEvent) -> Result<(), Error> {
        let button = event.detail;
        let mods = key_mods(event.state);
        let scale = self.scale.get();

        // We use a delta of 120 per tick to match the behavior of Windows.
        let is_shift = mods.shift();
        let delta = match button {
            4 if is_shift => (-120.0, 0.0),
            4 => (0.0, -120.0),
            5 if is_shift => (120.0, 0.0),
            5 => (0.0, 120.0),
            6 => (-120.0, 0.0),
            7 => (120.0, 0.0),
            _ => return Err(anyhow!("unexpected mouse wheel button: {}", button)),
        };
        let mouse_event = MouseEvent {
            pos: Point::new(event.event_x as f64, event.event_y as f64).to_dp(scale),
            buttons: mouse_buttons(event.state),
            mods: key_mods(event.state),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: delta.into(),
        };

        self.with_handler(|h| h.wheel(&mouse_event));
        Ok(())
    }

    pub fn handle_motion_notify(
        &self,
        motion_notify: &xproto::MotionNotifyEvent,
    ) -> Result<(), Error> {
        let scale = self.scale.get();
        let mouse_event = MouseEvent {
            pos: Point::new(motion_notify.event_x as f64, motion_notify.event_y as f64)
                .to_dp(scale),
            buttons: mouse_buttons(motion_notify.state),
            mods: key_mods(motion_notify.state),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        };
        self.with_handler(|h| h.mouse_move(&mouse_event));
        Ok(())
    }

    pub fn handle_client_message(&self, client_message: &xproto::ClientMessageEvent) {
        // https://www.x.org/releases/X11R7.7/doc/libX11/libX11/libX11.html#id2745388
        // https://www.x.org/releases/X11R7.6/doc/xorg-docs/specs/ICCCM/icccm.html#window_deletion
        let atoms = self.app.atoms();
        if client_message.type_ == atoms.WM_PROTOCOLS && client_message.format == 32 {
            let protocol = client_message.data.as_data32()[0];
            if protocol == atoms.WM_DELETE_WINDOW {
                self.with_handler(|h| h.request_close());
            }
        }
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn handle_destroy_notify(&self, _destroy_notify: &xproto::DestroyNotifyEvent) {
        self.with_handler(|h| h.destroy());
    }

    pub fn handle_configure_notify(&self, event: &ConfigureNotifyEvent) -> Result<(), Error> {
        self.size_changed(Size::new(event.width as f64, event.height as f64))
    }

    pub fn handle_complete_notify(&self, event: &CompleteNotifyEvent) -> Result<(), Error> {
        if let Some(present) = borrow_mut!(self.present_data)?.as_mut() {
            // A little sanity check (which isn't worth an early return): we should only have
            // one present request in flight, so we should only get notified about the request
            // that we're waiting for.
            if present.waiting_on != Some(event.serial) {
                warn!(
                    "Got a notify for serial {}, but waiting on {:?}",
                    event.serial, present.waiting_on
                );
            }

            // Check whether we missed presenting on any frames.
            if let Some(last_msc) = present.last_msc {
                if last_msc.wrapping_add(1) != event.msc {
                    tracing::debug!(
                        "missed a present: msc went from {} to {}",
                        last_msc,
                        event.msc
                    );
                    if let Some(last_ust) = present.last_ust {
                        tracing::debug!("ust went from {} to {}", last_ust, event.ust);
                    }
                }
            }

            // Only store the last MSC if we're animating (if we aren't animating, missed MSCs
            // aren't interesting).
            present.last_msc = if present.needs_present {
                Some(event.msc)
            } else {
                None
            };
            present.last_ust = Some(event.ust);
            present.waiting_on = None;
        }

        if self.needs_present()? {
            self.render()?;
        }
        Ok(())
    }

    pub fn handle_idle_notify(&self, event: &IdleNotifyEvent) -> Result<(), Error> {
        if self.destroyed() {
            return Ok(());
        }

        let mut buffers = borrow_mut!(self.buffers)?;
        if buffers.all_pixmaps.contains(&event.pixmap) {
            buffers.idle_pixmaps.push(event.pixmap);
        } else {
            // We must have reallocated the buffers while this pixmap was busy, so free it now.
            // Regular freeing happens in `Buffers::free_pixmaps`.
            self.app.connection().free_pixmap(event.pixmap)?;
        }
        Ok(())
    }

    fn waiting_on_present(&self) -> Result<bool, Error> {
        Ok(borrow!(self.present_data)?
            .as_ref()
            .map(|p| p.waiting_on.is_some())
            .unwrap_or(false))
    }

    fn set_needs_present(&self, val: bool) -> Result<(), Error> {
        if let Some(present) = borrow_mut!(self.present_data)?.as_mut() {
            present.needs_present = val;
        }
        Ok(())
    }

    fn needs_present(&self) -> Result<bool, Error> {
        Ok(borrow!(self.present_data)?
            .as_ref()
            .map(|p| p.needs_present)
            .unwrap_or(false))
    }

    pub(crate) fn run_idle(&self) {
        let mut queue = Vec::new();
        std::mem::swap(&mut *self.idle_queue.lock().unwrap(), &mut queue);

        let mut needs_redraw = false;
        self.with_handler(|handler| {
            for callback in queue {
                match callback {
                    IdleKind::Callback(f) => {
                        f.call(handler);
                    }
                    IdleKind::Token(tok) => {
                        handler.idle(tok);
                    }
                    IdleKind::Redraw => {
                        needs_redraw = true;
                    }
                }
            }
        });

        if needs_redraw {
            if let Err(e) = self.redraw_now() {
                error!("Error redrawing: {}", e);
            }
        }
    }

    pub(crate) fn next_timeout(&self) -> Option<Instant> {
        self.timer_queue
            .lock()
            .unwrap()
            .peek()
            .map(|timer| timer.deadline())
    }

    pub(crate) fn run_timers(&self, now: Instant) {
        while let Some(deadline) = self.next_timeout() {
            if deadline > now {
                break;
            }
            // Remove the timer and get the token
            let token = self.timer_queue.lock().unwrap().pop().unwrap().token();
            self.with_handler(|h| h.timer(token));
        }
    }
}

impl Buffers {
    fn new(
        conn: &Rc<XCBConnection>,
        window_id: u32,
        buf_count: usize,
        width: u16,
        height: u16,
        depth: u8,
    ) -> Result<Buffers, Error> {
        let mut ret = Buffers {
            width,
            height,
            depth,
            idle_pixmaps: Vec::new(),
            all_pixmaps: Vec::new(),
        };
        ret.create_pixmaps(conn, window_id, buf_count)?;
        Ok(ret)
    }

    /// Frees all the X pixmaps that we hold.
    fn free_pixmaps(&mut self, conn: &Rc<XCBConnection>) {
        // We can't touch pixmaps if the present extension is waiting on them, so only free the
        // idle ones. We'll free the busy ones when we get notified that they're idle in `Window::handle_idle_notify`.
        for &p in &self.idle_pixmaps {
            log_x11!(conn.free_pixmap(p));
        }
        self.all_pixmaps.clear();
        self.idle_pixmaps.clear();
    }

    fn set_size(&mut self, conn: &Rc<XCBConnection>, window_id: u32, width: u16, height: u16) {
        // How big should the buffer be if we want at least x pixels? Rounding up to the next power
        // of 2 has the potential to waste 75% of our memory (factor 2 in both directions), so
        // instead we round up to the nearest number of the form 2^k or 3 * 2^k.
        fn next_size(x: u16) -> u16 {
            // We round up to the nearest multiple of `accuracy`, which is between x/2 and x/4.
            // Don't bother rounding to anything smaller than 32 = 2^(7-1).
            let accuracy = 1 << ((16 - x.leading_zeros()).max(7) - 2);
            let mask = accuracy - 1;
            (x + mask) & !mask
        }

        let width = next_size(width);
        let height = next_size(height);
        if (width, height) != (self.width, self.height) {
            let count = self.all_pixmaps.len();
            self.free_pixmaps(conn);
            self.width = width;
            self.height = height;
            log_x11!(self.create_pixmaps(conn, window_id, count));
        }
    }

    /// Creates a new pixmap for rendering to. The new pixmap will be first in line for rendering.
    fn create_pixmap(&mut self, conn: &Rc<XCBConnection>, window_id: u32) -> Result<Pixmap, Error> {
        let pixmap_id = conn.generate_id()?;
        conn.create_pixmap(self.depth, pixmap_id, window_id, self.width, self.height)?;
        self.all_pixmaps.push(pixmap_id);
        self.idle_pixmaps.push(pixmap_id);
        Ok(pixmap_id)
    }

    fn create_pixmaps(
        &mut self,
        conn: &Rc<XCBConnection>,
        window_id: u32,
        count: usize,
    ) -> Result<(), Error> {
        if !self.all_pixmaps.is_empty() {
            self.free_pixmaps(conn);
        }

        for _ in 0..count {
            self.create_pixmap(conn, window_id)?;
        }
        Ok(())
    }
}

impl PresentData {
    // We have already rendered into the active pixmap buffer. Present it to the
    // X server, and then rotate the buffers.
    fn present(
        &mut self,
        conn: &Rc<XCBConnection>,
        pixmap: Pixmap,
        window_id: u32,
        region: &Region,
        scale: Scale,
    ) -> Result<(), Error> {
        let x_rects: Vec<Rectangle> = region
            .rects()
            .iter()
            .map(|r| {
                let r = r.to_px(scale).round();
                Rectangle {
                    x: r.x0 as i16,
                    y: r.y0 as i16,
                    width: r.width() as u16,
                    height: r.height() as u16,
                }
            })
            .collect();

        conn.xfixes_set_region(self.region, &x_rects[..])?;
        conn.present_pixmap(
            window_id,
            pixmap,
            self.serial,
            // valid region of the pixmap
            self.region,
            // region of the window that must get updated
            self.region,
            // window-relative x-offset of the pixmap
            0,
            // window-relative y-offset of the pixmap
            0,
            // target CRTC
            x11rb::NONE,
            // wait fence
            x11rb::NONE,
            // idle fence
            x11rb::NONE,
            // present options
            x11rb::protocol::present::Option::NONE.into(),
            // target msc (0 means present at the next time that msc % divisor == remainder)
            0,
            // divisor
            1,
            // remainder
            0,
            // notifies
            &[],
        )?;
        self.waiting_on = Some(self.serial);
        self.serial += 1;
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
            warn!("unknown mouse button code {}", button);
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
        if mods & u16::from(*mask) != 0 {
            buttons.insert(*button);
        }
    }
    buttons
}

// Extracts the keyboard modifiers from, e.g., the `state` field of
// `xcb::xproto::ButtonPressEvent`
fn key_mods(mods: u16) -> Modifiers {
    let mut ret = Modifiers::default();
    let mut key_masks = [
        (xproto::ModMask::SHIFT, Modifiers::SHIFT),
        (xproto::ModMask::CONTROL, Modifiers::CONTROL),
        // X11's mod keys are configurable, but this seems
        // like a reasonable default for US keyboards, at least,
        // where the "windows" key seems to be MOD_MASK_4.
        (xproto::ModMask::M1, Modifiers::ALT),
        (xproto::ModMask::M2, Modifiers::NUM_LOCK),
        (xproto::ModMask::M4, Modifiers::META),
        (xproto::ModMask::LOCK, Modifiers::CAPS_LOCK),
    ];
    for (mask, modifiers) in &mut key_masks {
        if mods & u16::from(*mask) != 0 {
            ret |= *modifiers;
        }
    }
    ret
}

/// A handle that can get used to schedule an idle handler. Note that
/// this handle can be cloned and sent between threads.
#[derive(Clone)]
pub struct IdleHandle {
    queue: Arc<Mutex<Vec<IdleKind>>>,
    pipe: RawFd,
}

pub(crate) enum IdleKind {
    Callback(Box<dyn IdleCallback>),
    Token(IdleToken),
    Redraw,
}

impl IdleHandle {
    fn wake(&self) {
        loop {
            match nix::unistd::write(self.pipe, &[0]) {
                Err(nix::Error::Sys(nix::errno::Errno::EINTR)) => {}
                Err(nix::Error::Sys(nix::errno::Errno::EAGAIN)) => {}
                Err(e) => {
                    error!("Failed to write to idle pipe: {}", e);
                    break;
                }
                Ok(_) => {
                    break;
                }
            }
        }
    }

    pub(crate) fn schedule_redraw(&self) {
        self.queue.lock().unwrap().push(IdleKind::Redraw);
        self.wake();
    }

    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&mut dyn WinHandler) + Send + 'static,
    {
        self.queue
            .lock()
            .unwrap()
            .push(IdleKind::Callback(Box::new(callback)));
        self.wake();
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        self.queue.lock().unwrap().push(IdleKind::Token(token));
        self.wake();
    }
}

#[derive(Clone, Default)]
pub(crate) struct WindowHandle {
    id: u32,
    window: Weak<Window>,
}
impl PartialEq for WindowHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for WindowHandle {}

impl WindowHandle {
    fn new(id: u32, window: Weak<Window>) -> WindowHandle {
        WindowHandle { id, window }
    }

    pub fn show(&self) {
        if let Some(w) = self.window.upgrade() {
            w.show();
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn close(&self) {
        if let Some(w) = self.window.upgrade() {
            w.close();
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn resizable(&self, resizable: bool) {
        if let Some(w) = self.window.upgrade() {
            w.resizable(resizable);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        if let Some(w) = self.window.upgrade() {
            w.show_titlebar(show_titlebar);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn set_position(&self, position: Point) {
        if let Some(w) = self.window.upgrade() {
            w.set_position(position);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn get_position(&self) -> Point {
        if let Some(w) = self.window.upgrade() {
            w.get_position()
        } else {
            error!("Window {} has already been dropped", self.id);
            Point::new(0.0, 0.0)
        }
    }

    pub fn content_insets(&self) -> Insets {
        warn!("WindowHandle::content_insets unimplemented for X11 backend.");
        Insets::ZERO
    }

    pub fn set_size(&self, size: Size) {
        if let Some(w) = self.window.upgrade() {
            w.set_size(size);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn get_size(&self) -> Size {
        if let Some(w) = self.window.upgrade() {
            w.size().size_dp()
        } else {
            error!("Window {} has already been dropped", self.id);
            Size::ZERO
        }
    }

    pub fn set_window_state(&self, _state: window::WindowState) {
        warn!("WindowHandle::set_window_state is currently unimplemented for X11 backend.");
    }

    pub fn get_window_state(&self) -> window::WindowState {
        warn!("WindowHandle::get_window_state is currently unimplemented for X11 backend.");
        window::WindowState::Restored
    }

    pub fn handle_titlebar(&self, _val: bool) {
        warn!("WindowHandle::handle_titlebar is currently unimplemented for X11 backend.");
    }

    pub fn bring_to_front_and_focus(&self) {
        if let Some(w) = self.window.upgrade() {
            w.bring_to_front_and_focus();
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn request_anim_frame(&self) {
        if let Some(w) = self.window.upgrade() {
            w.request_anim_frame();
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn invalidate(&self) {
        if let Some(w) = self.window.upgrade() {
            w.invalidate();
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(w) = self.window.upgrade() {
            w.invalidate_rect(rect);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn set_title(&self, title: &str) {
        if let Some(w) = self.window.upgrade() {
            w.set_title(title);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn set_menu(&self, menu: Menu) {
        if let Some(w) = self.window.upgrade() {
            w.set_menu(menu);
        } else {
            error!("Window {} has already been dropped", self.id);
        }
    }

    pub fn text(&self) -> PietText {
        PietText::new()
    }

    pub fn add_text_field(&self) -> TextFieldToken {
        TextFieldToken::next()
    }

    pub fn remove_text_field(&self, token: TextFieldToken) {
        if let Some(window) = self.window.upgrade() {
            if window.active_text_field.get() == Some(token) {
                window.active_text_field.set(None)
            }
        }
    }

    pub fn set_focused_text_field(&self, active_field: Option<TextFieldToken>) {
        if let Some(window) = self.window.upgrade() {
            window.active_text_field.set(active_field);
        }
    }

    pub fn update_text_field(&self, _token: TextFieldToken, _update: Event) {
        // noop until we get a real text input implementation
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        if let Some(w) = self.window.upgrade() {
            let timer = Timer::new(deadline);
            w.timer_queue.lock().unwrap().push(timer);
            timer.token()
        } else {
            TimerToken::INVALID
        }
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(w) = self.window.upgrade() {
            w.set_cursor(cursor);
        }
    }

    pub fn make_cursor(&self, desc: &CursorDesc) -> Option<Cursor> {
        if let Some(w) = self.window.upgrade() {
            match w.app.render_argb32_pictformat_cursor() {
                None => {
                    warn!("Custom cursors are not supported by the X11 server");
                    None
                }
                Some(format) => {
                    let conn = w.app.connection();
                    let setup = &conn.setup();
                    let screen = &setup.roots[w.app.screen_num()];
                    match make_cursor(&**conn, setup.image_byte_order, screen.root, format, desc) {
                        // TODO: We 'leak' the cursor - nothing ever calls render_free_cursor
                        Ok(cursor) => Some(cursor),
                        Err(err) => {
                            error!("Failed to create custom cursor: {:?}", err);
                            None
                        }
                    }
                }
            }
        } else {
            None
        }
    }

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        // TODO(x11/file_dialogs): implement WindowHandle::open_file
        warn!("WindowHandle::open_file is currently unimplemented for X11 backend.");
        None
    }

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        // TODO(x11/file_dialogs): implement WindowHandle::save_as
        warn!("WindowHandle::save_as is currently unimplemented for X11 backend.");
        None
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        // TODO(x11/menus): implement WindowHandle::show_context_menu
        warn!("WindowHandle::show_context_menu is currently unimplemented for X11 backend.");
    }

    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.window.upgrade().map(|w| IdleHandle {
            queue: Arc::clone(&w.idle_queue),
            pipe: w.idle_pipe,
        })
    }

    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        if let Some(w) = self.window.upgrade() {
            Ok(w.get_scale()?)
        } else {
            error!("Window {} has already been dropped", self.id);
            Ok(Scale::new(1.0, 1.0))
        }
    }
}

#[cfg(feature = "raw-win-handle")]
unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = XcbHandle {
            window: self.id,
            ..XcbHandle::empty()
        };

        if let Some(window) = self.window.upgrade() {
            handle.connection = window.app.connection().get_raw_xcb_connection();
        } else {
            // Documentation for HasRawWindowHandle encourages filling in all fields possible,
            // leaving those empty that cannot be derived.
            error!("Failed to get XCBConnection, returning incomplete handle");
        }

        RawWindowHandle::Xcb(handle)
    }
}

fn make_cursor(
    conn: &XCBConnection,
    byte_order: X11ImageOrder,
    root_window: u32,
    argb32_format: Pictformat,
    desc: &CursorDesc,
) -> Result<Cursor, ReplyOrIdError> {
    // BEGIN: Lots of code just to get the image into a RENDER Picture

    fn multiply_alpha(color: u8, alpha: u8) -> u8 {
        let (color, alpha) = (u16::from(color), u16::from(alpha));
        let temp = color * alpha + 0x80u16;
        ((temp + (temp >> 8)) >> 8) as u8
    }

    // No idea how to sanely get the pixel values, so I'll go with 'insane':
    // Iterate over all pixels and build an array
    let pixels = desc
        .image
        .pixel_colors()
        .flat_map(|row| {
            row.flat_map(|color| {
                let (r, g, b, a) = color.as_rgba8();
                // RENDER wants premultiplied alpha
                let (r, g, b) = (
                    multiply_alpha(r, a),
                    multiply_alpha(g, a),
                    multiply_alpha(b, a),
                );
                // piet gives us rgba in this order, the server expects an u32 with argb.
                let (b0, b1, b2, b3) = match byte_order {
                    X11ImageOrder::LSB_FIRST => (b, g, r, a),
                    _ => (a, r, g, b),
                };
                // TODO Ownership and flat_map don't go well together :-(
                vec![b0, b1, b2, b3]
            })
        })
        .collect::<Vec<u8>>();
    let width = desc.image.width().try_into().expect("Invalid cursor width");
    let height = desc
        .image
        .height()
        .try_into()
        .expect("Invalid cursor height");

    let pixmap = conn.generate_id()?;
    let gc = conn.generate_id()?;
    let picture = conn.generate_id()?;
    conn.create_pixmap(32, pixmap, root_window, width, height)?;
    conn.create_gc(gc, pixmap, &Default::default())?;

    conn.put_image(
        ImageFormat::Z_PIXMAP,
        pixmap,
        gc,
        width,
        height,
        0,
        0,
        0,
        32,
        &pixels,
    )?;
    conn.render_create_picture(picture, pixmap, argb32_format, &Default::default())?;

    conn.free_gc(gc)?;
    conn.free_pixmap(pixmap)?;
    // End: Lots of code just to get the image into a RENDER Picture

    let cursor = conn.generate_id()?;
    conn.render_create_cursor(cursor, picture, desc.hot.x as u16, desc.hot.y as u16)?;
    conn.render_free_picture(picture)?;

    Ok(Cursor::Custom(CustomCursor(cursor)))
}
