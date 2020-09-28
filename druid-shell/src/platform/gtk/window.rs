// Copyright 2019 The Druid Authors.
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

//! GTK window creation and management.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::convert::TryFrom;
use std::ffi::c_void;
use std::ffi::OsString;
use std::os::raw::{c_int, c_uint};
use std::ptr;
use std::slice;
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use anyhow::anyhow;
use cairo::Surface;
use gdk::{EventKey, EventMask, ModifierType, ScrollDirection, WindowExt, WindowTypeHint};
use gio::ApplicationExt;
use gtk::prelude::*;
use gtk::{AccelGroup, ApplicationWindow, DrawingArea};

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::{Piet, PietText, RenderContext};

use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::error::Error as ShellError;
use crate::keyboard::{KbKey, KeyEvent, KeyState, Modifiers};
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
use crate::region::Region;
use crate::scale::{Scalable, Scale, ScaledArea};
use crate::window;
use crate::window::{IdleToken, TimerToken, WinHandler, WindowLevel};

use super::application::Application;
use super::dialog;
use super::keycodes;
use super::menu::Menu;
use super::util;

/// The platform target DPI.
///
/// GTK considers 96 the default value which represents a 1.0 scale factor.
const SCALE_TARGET_DPI: f64 = 96.0;

/// Taken from https://gtk-rs.org/docs-src/tutorial/closures
/// It is used to reduce the boilerplate of setting up gtk callbacks
/// Example:
/// ```
/// button.connect_clicked(clone!(handle => move |_| { ... }))
/// ```
/// is equivalent to:
/// ```
/// {
///     let handle = handle.clone();
///     button.connect_clicked(move |_| { ... })
/// }
/// ```
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

#[derive(Clone, Default)]
pub struct WindowHandle {
    pub(crate) state: Weak<WindowState>,
}

/// Builder abstraction for creating new windows
pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    position: Option<Point>,
    level: Option<WindowLevel>,
    state: Option<window::WindowState>,
    size: Size,
    min_size: Option<Size>,
    resizable: bool,
    show_titlebar: bool,
}

#[derive(Clone)]
pub struct IdleHandle {
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    state: Weak<WindowState>,
}

/// This represents different Idle Callback Mechanism
enum IdleKind {
    Callback(Box<dyn IdleCallback>),
    Token(IdleToken),
}

pub(crate) struct WindowState {
    window: ApplicationWindow,
    scale: Cell<Scale>,
    area: Cell<ScaledArea>,
    /// Used to determine whether to honor close requests from the system: we inhibit them unless
    /// this is true, and this gets set to true when our client requests a close.
    closing: Cell<bool>,
    drawing_area: DrawingArea,
    // A cairo surface for us to render to; we copy this to the drawing_area whenever necessary.
    // This extra buffer is necessitated by DrawingArea's painting model: when our paint callback
    // is called, we are given a cairo context that's already clipped to the invalid region. This
    // doesn't match up with our painting model, because we need to call `prepare_paint` before we
    // know what the invalid region is.
    //
    // The way we work around this is by always invalidating the entire DrawingArea whenever we
    // need repainting; this ensures that GTK gives us an unclipped cairo context. Meanwhile, we
    // keep track of the actual invalid region. We use that region to render onto `surface`, which
    // we then copy onto `drawing_area`.
    surface: RefCell<Option<Surface>>,
    // The size of `surface` in pixels. This could be bigger than `drawing_area`.
    surface_size: RefCell<(i32, i32)>,
    // The invalid region, in display points.
    invalid: RefCell<Region>,
    pub(crate) handler: RefCell<Box<dyn WinHandler>>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    current_keycode: RefCell<Option<u16>>,
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(500.0, 400.0),
            position: None,
            level: None,
            state: None,
            min_size: None,
            resizable: true,
            show_titlebar: true,
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = Some(size);
    }

    pub fn resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    pub fn show_titlebar(&mut self, show_titlebar: bool) {
        self.show_titlebar = show_titlebar;
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = Some(position);
    }

    pub fn set_level(&mut self, level: WindowLevel) {
        self.level = Some(level);
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        self.state = Some(state);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, ShellError> {
        let handler = self
            .handler
            .expect("Tried to build a window without setting the handler");

        let window = ApplicationWindow::new(self.app.gtk_app());

        window.set_title(&self.title);
        window.set_resizable(self.resizable);
        window.set_decorated(self.show_titlebar);

        // Get the scale factor based on the GTK reported DPI
        let scale_factor =
            window.get_display().get_default_screen().get_resolution() / SCALE_TARGET_DPI;
        let scale = Scale::new(scale_factor, scale_factor);
        let area = ScaledArea::from_dp(self.size, scale);
        let size_px = area.size_px();

        window.set_default_size(size_px.width as i32, size_px.height as i32);

        let accel_group = AccelGroup::new();
        window.add_accel_group(&accel_group);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&vbox);
        let drawing_area = gtk::DrawingArea::new();

        let win_state = Arc::new(WindowState {
            window,
            scale: Cell::new(scale),
            area: Cell::new(area),
            closing: Cell::new(false),
            drawing_area,
            surface: RefCell::new(None),
            surface_size: RefCell::new((0, 0)),
            invalid: RefCell::new(Region::EMPTY),
            handler: RefCell::new(handler),
            idle_queue: Arc::new(Mutex::new(vec![])),
            current_keycode: RefCell::new(None),
        });

        self.app
            .gtk_app()
            .connect_shutdown(clone!(win_state => move |_| {
                // this ties a clone of Arc<WindowState> to the ApplicationWindow to keep it alive
                // when the ApplicationWindow is destroyed, the last Arc is dropped
                // and any Weak<WindowState> will be None on upgrade()
                let _ = &win_state;
            }));

        let mut handle = WindowHandle {
            state: Arc::downgrade(&win_state),
        };
        if let Some(level) = self.level {
            handle.set_level(level);
        }
        if let Some(pos) = self.position {
            handle.set_position(pos);
        }
        if let Some(state) = self.state {
            handle.set_window_state(state)
        }

        if let Some(menu) = self.menu {
            let menu = menu.into_gtk_menubar(&handle, &accel_group);
            vbox.pack_start(&menu, false, false, 0);
        }

        win_state.drawing_area.set_events(
            EventMask::EXPOSURE_MASK
                | EventMask::POINTER_MOTION_MASK
                | EventMask::LEAVE_NOTIFY_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::BUTTON_RELEASE_MASK
                | EventMask::KEY_PRESS_MASK
                | EventMask::ENTER_NOTIFY_MASK
                | EventMask::KEY_RELEASE_MASK
                | EventMask::SCROLL_MASK
                | EventMask::SMOOTH_SCROLL_MASK,
        );

        win_state.drawing_area.set_can_focus(true);
        win_state.drawing_area.grab_focus();

        win_state
            .drawing_area
            .connect_enter_notify_event(|widget, _| {
                widget.grab_focus();

                Inhibit(true)
            });

        // Set the minimum size
        if let Some(min_size_dp) = self.min_size {
            let min_area = ScaledArea::from_dp(min_size_dp, scale);
            let min_size_px = min_area.size_px();
            win_state
                .drawing_area
                .set_size_request(min_size_px.width as i32, min_size_px.height as i32);
        }

        win_state.drawing_area.connect_draw(clone!(handle => move |widget, context| {
            if let Some(state) = handle.state.upgrade() {
                let mut scale = state.scale.get();
                let mut scale_changed = false;
                // Check if the GTK reported DPI has changed,
                // so that we can change our scale factor without restarting the application.
                if let Some(scale_factor) = state.window.get_window()
                    .map(|w| w.get_display().get_default_screen().get_resolution() / SCALE_TARGET_DPI) {
                    let reported_scale = Scale::new(scale_factor, scale_factor);
                    if scale != reported_scale {
                        scale = reported_scale;
                        state.scale.set(scale);
                        scale_changed = true;
                        if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                            handler_borrow.scale(scale);
                        } else {
                            log::warn!("Failed to inform the handler of scale change because it was already borrowed");
                        }
                    }
                }

                // Create a new cairo surface if necessary (either because there is no surface, or
                // because the size or scale changed).
                let extents = widget.get_allocation();
                let size_px = Size::new(extents.width as f64, extents.height as f64);
                let no_surface = state.surface.try_borrow().map(|x| x.is_none()).ok() == Some(true);
                if no_surface || scale_changed || state.area.get().size_px() != size_px {
                    let area = ScaledArea::from_px(size_px, scale);
                    let size_dp = area.size_dp();
                    state.area.set(area);
                    if let Err(e) = state.resize_surface(extents.width, extents.height) {
                        log::error!("Failed to resize surface: {}", e);
                    }
                    if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                        handler_borrow.size(size_dp);
                    } else {
                        log::warn!("Failed to inform the handler of a resize because it was already borrowed");
                    }
                    state.invalidate_rect(size_dp.to_rect());
                }

                if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                    // Note that we aren't holding any RefCell borrows here (except for the
                    // WinHandler itself), because prepare_paint can call back into our WindowHandle
                    // (most likely for invalidation).
                    handler_borrow.prepare_paint();

                    let surface = state.surface.try_borrow();
                    if let Ok(Some(surface)) = surface.as_ref().map(|s| s.as_ref()) {
                        if let Ok(mut invalid) = state.invalid.try_borrow_mut() {
                            let surface_context = cairo::Context::new(surface);

                            // Clip to the invalid region, in order that our surface doesn't get
                            // messed up if there's any painting outside them.
                            for rect in invalid.rects() {
                                surface_context.rectangle(rect.x0, rect.y0, rect.width(), rect.height());
                            }
                            surface_context.clip();

                            surface_context.scale(scale.x(), scale.y());
                            let mut piet_context = Piet::new(&surface_context);
                            handler_borrow.paint(&mut piet_context, &invalid);
                            if let Err(e) = piet_context.finish() {
                                log::error!("piet error on render: {:?}", e);
                            }

                            // Copy the entire surface to the drawing area (not just the invalid
                            // region, because there might be parts of the drawing area that were
                            // invalidated by external forces).
                            let alloc = widget.get_allocation();
                            context.set_source_surface(&surface, 0.0, 0.0);
                            context.rectangle(0.0, 0.0, alloc.width as f64, alloc.height as f64);
                            context.fill();

                            invalid.clear();
                        } else {
                            log::warn!("Drawing was skipped because the invalid region was borrowed");
                        }
                    } else {
                        log::warn!("Drawing was skipped because there was no surface");
                    }
                } else {
                    log::warn!("Drawing was skipped because the handler was already borrowed");
                }
            }

            Inhibit(false)
        }));

        win_state.drawing_area.connect_button_press_event(clone!(handle => move |_widget, event| {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    if let Some(button) = get_mouse_button(event.get_button()) {
                        let scale = state.scale.get();
                        let button_state = event.get_state();
                        handler.mouse_down(
                            &MouseEvent {
                                pos: Point::from(event.get_position()).to_dp(scale),
                                buttons: get_mouse_buttons_from_modifiers(button_state).with(button),
                                mods: get_modifiers(button_state),
                                count: get_mouse_click_count(event.get_event_type()),
                                focus: false,
                                button,
                                wheel_delta: Vec2::ZERO
                            },
                        );
                    }
                } else {
                    log::warn!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_button_release_event(clone!(handle => move |_widget, event| {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    if let Some(button) = get_mouse_button(event.get_button()) {
                        let scale = state.scale.get();
                        let button_state = event.get_state();
                        handler.mouse_up(
                            &MouseEvent {
                                pos: Point::from(event.get_position()).to_dp(scale),
                                buttons: get_mouse_buttons_from_modifiers(button_state).without(button),
                                mods: get_modifiers(button_state),
                                count: 0,
                                focus: false,
                                button,
                                wheel_delta: Vec2::ZERO
                            },
                        );
                    }
                } else {
                    log::warn!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_motion_notify_event(clone!(handle => move |_widget, motion| {
            if let Some(state) = handle.state.upgrade() {
                let scale = state.scale.get();
                let motion_state = motion.get_state();
                let mouse_event = MouseEvent {
                    pos: Point::from(motion.get_position()).to_dp(scale),
                    buttons: get_mouse_buttons_from_modifiers(motion_state),
                    mods: get_modifiers(motion_state),
                    count: 0,
                    focus: false,
                    button: MouseButton::None,
                    wheel_delta: Vec2::ZERO
                };

                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    handler.mouse_move(&mouse_event);
                } else {
                    log::warn!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_leave_notify_event(clone!(handle => move |_widget, crossing| {
            if let Some(state) = handle.state.upgrade() {
                let scale = state.scale.get();
                let crossing_state = crossing.get_state();
                let mouse_event = MouseEvent {
                    pos: Point::from(crossing.get_position()).to_dp(scale),
                    buttons: get_mouse_buttons_from_modifiers(crossing_state),
                    mods: get_modifiers(crossing_state),
                    count: 0,
                    focus: false,
                    button: MouseButton::None,
                    wheel_delta: Vec2::ZERO
                };

                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    handler.mouse_move(&mouse_event);
                } else {
                    log::warn!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_scroll_event(clone!(handle => move |_widget, scroll| {
            if let Some(state) = handle.state.upgrade() {
                let scale = state.scale.get();
                let mods = get_modifiers(scroll.get_state());

                // The magic "120"s are from Microsoft's documentation for WM_MOUSEWHEEL.
                // They claim that one "tick" on a scroll wheel should be 120 units.
                let shift = mods.shift();
                let wheel_delta = match scroll.get_direction() {
                    ScrollDirection::Up if shift => Some(Vec2::new(-120.0, 0.0)),
                    ScrollDirection::Up => Some(Vec2::new(0.0, -120.0)),
                    ScrollDirection::Down if shift => Some(Vec2::new(120.0, 0.0)),
                    ScrollDirection::Down => Some(Vec2::new(0.0, 120.0)),
                    ScrollDirection::Left => Some(Vec2::new(-120.0, 0.0)),
                    ScrollDirection::Right => Some(Vec2::new(120.0, 0.0)),
                    ScrollDirection::Smooth => {
                        //TODO: Look at how gtk's scroll containers implements it
                        let (mut delta_x, mut delta_y) = scroll.get_delta();
                        delta_x *= 120.;
                        delta_y *= 120.;
                        if shift {
                            delta_x += delta_y;
                            delta_y = 0.;
                        }
                        Some(Vec2::new(delta_x, delta_y))
                    }
                    e => {
                        eprintln!(
                            "Warning: the Druid widget got some whacky scroll direction {:?}",
                            e
                        );
                        None
                    }
                };

                if let Some(wheel_delta) = wheel_delta {
                    let mouse_event = MouseEvent {
                        pos: Point::from(scroll.get_position()).to_dp(scale),
                        buttons: get_mouse_buttons_from_modifiers(scroll.get_state()),
                        mods,
                        count: 0,
                        focus: false,
                        button: MouseButton::None,
                        wheel_delta
                    };

                    if let Ok(mut handler) = state.handler.try_borrow_mut() {
                        handler.wheel(&mouse_event);
                    } else {
                        log::info!("GTK event was dropped because the handler was already borrowed");
                    }
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_key_press_event(clone!(handle => move |_widget, key| {
            if let Some(state) = handle.state.upgrade() {

                let mut current_keycode = state.current_keycode.borrow_mut();
                let hw_keycode = key.get_hardware_keycode();
                let repeat = *current_keycode == Some(hw_keycode);

                *current_keycode = Some(hw_keycode);

                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    handler.key_down(make_key_event(key, repeat, KeyState::Down));
                } else {
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_key_release_event(clone!(handle => move |_widget, key| {
            if let Some(state) = handle.state.upgrade() {

                let mut current_keycode = state.current_keycode.borrow_mut();
                if *current_keycode == Some(key.get_hardware_keycode()) {
                    *current_keycode = None;
                }

                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    handler.key_up(make_key_event(key, false, KeyState::Up));
                } else {
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state
            .window
            .connect_delete_event(clone!(handle => move |_widget, _ev| {
                if let Some(state) = handle.state.upgrade() {
                    state.handler.borrow_mut().request_close();
                    Inhibit(!state.closing.get())
                } else {
                    Inhibit(false)
                }
            }));

        win_state
            .drawing_area
            .connect_destroy(clone!(handle => move |_widget| {
                if let Some(state) = handle.state.upgrade() {
                    state.handler.borrow_mut().destroy();
                }
            }));

        vbox.pack_end(&win_state.drawing_area, true, true, 0);
        win_state.drawing_area.realize();
        win_state
            .drawing_area
            .get_window()
            .expect("realize didn't create window")
            .set_event_compression(false);

        let mut handler = win_state.handler.borrow_mut();
        handler.connect(&handle.clone().into());
        handler.scale(scale);
        handler.size(self.size);

        Ok(handle)
    }
}

impl WindowState {
    fn resize_surface(&self, width: i32, height: i32) -> Result<(), anyhow::Error> {
        fn next_size(x: i32) -> i32 {
            // We round up to the nearest multiple of `accuracy`, which is between x/2 and x/4.
            // Don't bother rounding to anything smaller than 32 = 2^(7-1).
            let accuracy = 1 << ((32 - x.leading_zeros()).max(7) - 2);
            let mask = accuracy - 1;
            (x + mask) & !mask
        }

        let mut surface = self.surface.borrow_mut();
        let mut cur_size = self.surface_size.borrow_mut();
        let (width, height) = (next_size(width), next_size(height));
        if surface.is_none() || *cur_size != (width, height) {
            *cur_size = (width, height);
            if let Some(s) = surface.as_ref() {
                s.finish();
            }
            *surface = None;

            if let Some(w) = self.drawing_area.get_window() {
                *surface = w.create_similar_surface(cairo::Content::Color, width, height);
                if surface.is_none() {
                    return Err(anyhow!("create_similar_surface failed"));
                }
            } else {
                return Err(anyhow!("drawing area has no window"));
            }
        }
        Ok(())
    }

    /// Queues a call to `prepare_paint` and `paint`, but without marking any region for
    /// invalidation.
    fn request_anim_frame(&self) {
        self.window.queue_draw();
    }

    /// Invalidates a rectangle, given in display points.
    fn invalidate_rect(&self, rect: Rect) {
        if let Ok(mut region) = self.invalid.try_borrow_mut() {
            let scale = self.scale.get();
            // We prefer to invalidate an integer number of pixels.
            let rect = rect.to_px(scale).expand().to_dp(scale);
            region.add_rect(rect);
            self.window.queue_draw();
        } else {
            log::warn!("Not invalidating rect because region already borrowed");
        }
    }
}

impl WindowHandle {
    pub fn show(&self) {
        if let Some(state) = self.state.upgrade() {
            state.window.show_all();
        }
    }

    pub fn resizable(&self, resizable: bool) {
        if let Some(state) = self.state.upgrade() {
            state.window.set_resizable(resizable)
        }
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        if let Some(state) = self.state.upgrade() {
            state.window.set_decorated(show_titlebar)
        }
    }

    pub fn set_position(&self, position: Point) {
        if let Some(state) = self.state.upgrade() {
            state.window.move_(position.x as i32, position.y as i32)
        }
    }

    pub fn get_position(&self) -> Point {
        if let Some(state) = self.state.upgrade() {
            let (x, y) = state.window.get_position();
            Point::new(x as f64, y as f64)
        } else {
            Point::new(0.0, 0.0)
        }
    }

    pub fn set_level(&self, level: WindowLevel) {
        if let Some(state) = self.state.upgrade() {
            let hint = match level {
                WindowLevel::AppWindow => WindowTypeHint::Normal,
                WindowLevel::Tooltip => WindowTypeHint::Tooltip,
                WindowLevel::DropDown => WindowTypeHint::DropdownMenu,
                WindowLevel::Modal => WindowTypeHint::Dialog,
            };

            state.window.set_type_hint(hint);
        }
    }

    pub fn set_size(&self, size: Size) {
        if let Some(state) = self.state.upgrade() {
            state.window.resize(size.width as i32, size.height as i32)
        }
    }

    pub fn get_size(&self) -> Size {
        if let Some(state) = self.state.upgrade() {
            let (x, y) = state.window.get_size();
            Size::new(x as f64, y as f64)
        } else {
            log::warn!("Could not get size for GTK window");
            Size::new(0., 0.)
        }
    }

    pub fn set_window_state(&mut self, size_state: window::WindowState) {
        use window::WindowState::{MAXIMIZED, MINIMIZED, RESTORED};
        let cur_size_state = self.get_window_state();
        if let Some(state) = self.state.upgrade() {
            match (size_state, cur_size_state) {
                (s1, s2) if s1 == s2 => (),
                (MAXIMIZED, _) => state.window.maximize(),
                (MINIMIZED, _) => state.window.iconify(),
                (RESTORED, MAXIMIZED) => state.window.unmaximize(),
                (RESTORED, MINIMIZED) => state.window.deiconify(),
                (RESTORED, RESTORED) => (), // Unreachable
            }

            state.window.unmaximize();
        }
    }

    pub fn get_window_state(&self) -> window::WindowState {
        use window::WindowState::{MAXIMIZED, MINIMIZED, RESTORED};
        if let Some(state) = self.state.upgrade() {
            if state.window.is_maximized() {
                return MAXIMIZED;
            } else if let Some(window) = state.window.get_parent_window() {
                let state = window.get_state();
                if (state & gdk::WindowState::ICONIFIED) == gdk::WindowState::ICONIFIED {
                    return MINIMIZED;
                }
            }
        }
        RESTORED
    }

    pub fn handle_titlebar(&self, _val: bool) {
        log::warn!("WindowHandle::handle_titlebar is currently unimplemented for gtk.");
    }

    /// Close the window.
    pub fn close(&self) {
        if let Some(state) = self.state.upgrade() {
            state.closing.set(true);
            state.window.close();
        }
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        if let Some(state) = self.state.upgrade() {
            // TODO(gtk/misc): replace with present_with_timestamp if/when druid-shell
            // has a system to get the correct input time, as GTK discourages present
            state.window.present();
        }
    }

    /// Request a new paint, but without invalidating anything.
    pub fn request_anim_frame(&self) {
        if let Some(state) = self.state.upgrade() {
            state.request_anim_frame();
        }
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        if let Some(state) = self.state.upgrade() {
            self.invalidate_rect(state.area.get().size_dp().to_rect());
        }
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(state) = self.state.upgrade() {
            state.invalidate_rect(rect);
        }
    }

    pub fn text(&self) -> PietText {
        PietText::new()
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        let interval = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default()
            .as_millis();
        let interval = match u32::try_from(interval) {
            Ok(iv) => iv,
            Err(_) => {
                log::warn!("timer duration exceeds gtk max of 2^32 millis");
                u32::max_value()
            }
        };

        let token = TimerToken::next();
        let handle = self.clone();

        glib::timeout_add(interval, move || {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                    handler_borrow.timer(token);
                    return glib::Continue(false);
                }
            }
            glib::Continue(true)
        });
        token
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(gdk_window) = self.state.upgrade().and_then(|s| s.window.get_window()) {
            let cursor = make_gdk_cursor(cursor, &gdk_window);
            gdk_window.set_cursor(cursor.as_ref());
        }
    }

    pub fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        self.file_dialog(FileDialogType::Open, options)
            .ok()
            .map(|s| FileInfo { path: s.into() })
    }

    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        self.file_dialog(FileDialogType::Save, options)
            .ok()
            .map(|s| FileInfo { path: s.into() })
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.state.upgrade().map(|s| IdleHandle {
            idle_queue: s.idle_queue.clone(),
            state: Arc::downgrade(&s),
        })
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        Ok(self
            .state
            .upgrade()
            .ok_or(ShellError::WindowDropped)?
            .scale
            .get())
    }

    pub fn set_menu(&self, menu: Menu) {
        if let Some(state) = self.state.upgrade() {
            let window = &state.window;

            let accel_group = AccelGroup::new();
            window.add_accel_group(&accel_group);

            let vbox = window.get_children()[0]
                .clone()
                .downcast::<gtk::Box>()
                .unwrap();

            let first_child = &vbox.get_children()[0];
            if first_child.is::<gtk::MenuBar>() {
                vbox.remove(first_child);
            }
            let menubar = menu.into_gtk_menubar(&self, &accel_group);
            vbox.pack_start(&menubar, false, false, 0);
            menubar.show_all();
        }
    }

    pub fn show_context_menu(&self, menu: Menu, _pos: Point) {
        if let Some(state) = self.state.upgrade() {
            let window = &state.window;

            let accel_group = AccelGroup::new();
            window.add_accel_group(&accel_group);

            let menu = menu.into_gtk_menu(&self, &accel_group);
            menu.set_property_attach_widget(Some(window));
            menu.show_all();
            menu.popup_easy(3, gtk::get_current_event_time());
        }
    }

    pub fn set_title(&self, title: impl Into<String>) {
        if let Some(state) = self.state.upgrade() {
            state.window.set_title(&(title.into()));
        }
    }

    fn file_dialog(
        &self,
        ty: FileDialogType,
        options: FileDialogOptions,
    ) -> Result<OsString, ShellError> {
        if let Some(state) = self.state.upgrade() {
            dialog::get_file_dialog_path(state.window.upcast_ref(), ty, options)
        } else {
            Err(anyhow!("Cannot upgrade state from weak pointer to arc").into())
        }
    }
}

unsafe impl Send for IdleHandle {}
// WindowState needs to be Send + Sync so it can be passed into glib closures
unsafe impl Send for WindowState {}
unsafe impl Sync for WindowState {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the main UI thread, and
    /// won't be scheduled if the associated view has been dropped.
    ///
    /// Note: the name "idle" suggests that it will be scheduled with a lower
    /// priority than other UI events, but that's not necessarily the case.
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        let mut queue = self.idle_queue.lock().unwrap();
        if let Some(state) = self.state.upgrade() {
            if queue.is_empty() {
                queue.push(IdleKind::Callback(Box::new(callback)));
                glib::idle_add(move || run_idle(&state));
            } else {
                queue.push(IdleKind::Callback(Box::new(callback)));
            }
        }
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        let mut queue = self.idle_queue.lock().unwrap();
        if let Some(state) = self.state.upgrade() {
            if queue.is_empty() {
                queue.push(IdleKind::Token(token));
                glib::idle_add(move || run_idle(&state));
            } else {
                queue.push(IdleKind::Token(token));
            }
        }
    }
}

fn run_idle(state: &Arc<WindowState>) -> glib::source::Continue {
    util::assert_main_thread();
    if let Ok(mut handler) = state.handler.try_borrow_mut() {
        let queue: Vec<_> = std::mem::replace(&mut state.idle_queue.lock().unwrap(), Vec::new());

        for item in queue {
            match item {
                IdleKind::Callback(it) => it.call(handler.as_any()),
                IdleKind::Token(it) => handler.idle(it),
            }
        }
    } else {
        log::warn!("Delaying idle callbacks because the handler is borrowed.");
        // Keep trying to reschedule this idle callback, because we haven't had a chance
        // to empty the idle queue. Returning glib::source::Continue(true) achieves this but
        // causes 100% CPU usage, apparently because glib likes to call us back very quickly.
        let state = Arc::clone(state);
        glib::timeout_add(16, move || run_idle(&state));
    }
    glib::source::Continue(false)
}

fn make_gdk_cursor(cursor: &Cursor, gdk_window: &gdk::Window) -> Option<gdk::Cursor> {
    gdk::Cursor::from_name(
        &gdk_window.get_display(),
        match cursor {
            // cursor name values from https://www.w3.org/TR/css-ui-3/#cursor
            Cursor::Arrow => "default",
            Cursor::IBeam => "text",
            Cursor::Crosshair => "crosshair",
            Cursor::OpenHand => "grab",
            Cursor::NotAllowed => "not-allowed",
            Cursor::ResizeLeftRight => "ew-resize",
            Cursor::ResizeUpDown => "ns-resize",
        },
    )
}

fn get_mouse_button(button: u32) -> Option<MouseButton> {
    match button {
        1 => Some(MouseButton::Left),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::Right),
        // GDK X backend interprets button press events for button 4-7 as scroll events
        8 => Some(MouseButton::X1),
        9 => Some(MouseButton::X2),
        _ => None,
    }
}

fn get_mouse_buttons_from_modifiers(modifiers: gdk::ModifierType) -> MouseButtons {
    let mut buttons = MouseButtons::new();
    if modifiers.contains(ModifierType::BUTTON1_MASK) {
        buttons.insert(MouseButton::Left);
    }
    if modifiers.contains(ModifierType::BUTTON2_MASK) {
        buttons.insert(MouseButton::Middle);
    }
    if modifiers.contains(ModifierType::BUTTON3_MASK) {
        buttons.insert(MouseButton::Right);
    }
    // TODO: Determine X1/X2 state (do caching ourselves if needed)
    //       Checking for BUTTON4_MASK/BUTTON5_MASK does not work with GDK X,
    //       because those are wheel events instead.
    if modifiers.contains(ModifierType::BUTTON4_MASK) {
        buttons.insert(MouseButton::X1);
    }
    if modifiers.contains(ModifierType::BUTTON5_MASK) {
        buttons.insert(MouseButton::X2);
    }
    buttons
}

fn get_mouse_click_count(event_type: gdk::EventType) -> u8 {
    match event_type {
        gdk::EventType::ButtonPress => 1,
        gdk::EventType::DoubleButtonPress => 2,
        gdk::EventType::TripleButtonPress => 3,
        gdk::EventType::ButtonRelease => 0,
        _ => {
            log::warn!("Unexpected mouse click event type: {:?}", event_type);
            0
        }
    }
}

const MODIFIER_MAP: &[(gdk::ModifierType, Modifiers)] = &[
    (ModifierType::SHIFT_MASK, Modifiers::SHIFT),
    (ModifierType::MOD1_MASK, Modifiers::ALT),
    (ModifierType::CONTROL_MASK, Modifiers::CONTROL),
    (ModifierType::META_MASK, Modifiers::META),
    (ModifierType::LOCK_MASK, Modifiers::CAPS_LOCK),
    // Note: this is the usual value on X11, not sure how consistent it is.
    // Possibly we should use `Keymap::get_num_lock_state()` instead.
    (ModifierType::MOD2_MASK, Modifiers::NUM_LOCK),
];

fn get_modifiers(modifiers: gdk::ModifierType) -> Modifiers {
    let mut result = Modifiers::empty();
    for &(gdk_mod, modifier) in MODIFIER_MAP {
        if modifiers.contains(gdk_mod) {
            result |= modifier;
        }
    }
    result
}

fn make_key_event(key: &EventKey, repeat: bool, state: KeyState) -> KeyEvent {
    let keyval = key.get_keyval();
    let hardware_keycode = key.get_hardware_keycode();

    let keycode = hardware_keycode_to_keyval(hardware_keycode).unwrap_or_else(|| keyval.clone());

    let text = gdk::keys::keyval_to_unicode(*keyval);
    let mods = get_modifiers(key.get_state());
    let key = keycodes::raw_key_to_key(keyval).unwrap_or_else(|| {
        if let Some(c) = text {
            if c >= ' ' && c != '\x7f' {
                KbKey::Character(c.to_string())
            } else {
                KbKey::Unidentified
            }
        } else {
            KbKey::Unidentified
        }
    });
    let code = keycodes::hardware_keycode_to_code(hardware_keycode);
    let location = keycodes::raw_key_to_location(keycode);
    let is_composing = false;

    KeyEvent {
        key,
        code,
        location,
        mods,
        repeat,
        is_composing,
        state,
    }
}

/// Map a hardware keycode to a keyval by performing a lookup in the keymap and finding the
/// keyval with the lowest group and level
fn hardware_keycode_to_keyval(keycode: u16) -> Option<keycodes::RawKey> {
    use glib::translate::FromGlib;
    unsafe {
        let keymap = gdk_sys::gdk_keymap_get_default();

        let mut nkeys = 0;
        let mut keys: *mut gdk_sys::GdkKeymapKey = ptr::null_mut();
        let mut keyvals: *mut c_uint = ptr::null_mut();

        // call into gdk to retrieve the keyvals and keymap keys
        gdk_sys::gdk_keymap_get_entries_for_keycode(
            keymap,
            c_uint::from(keycode),
            &mut keys as *mut *mut gdk_sys::GdkKeymapKey,
            &mut keyvals as *mut *mut c_uint,
            &mut nkeys as *mut c_int,
        );

        if nkeys > 0 {
            let keyvals_slice = slice::from_raw_parts(keyvals, nkeys as usize);
            let keys_slice = slice::from_raw_parts(keys, nkeys as usize);

            let resolved_keyval = keys_slice.iter().enumerate().find_map(|(i, key)| {
                if key.group == 0 && key.level == 0 {
                    Some(keycodes::RawKey::from_glib(keyvals_slice[i]))
                } else {
                    None
                }
            });

            // notify glib to free the allocated arrays
            glib_sys::g_free(keyvals as *mut c_void);
            glib_sys::g_free(keys as *mut c_void);

            resolved_keyval
        } else {
            None
        }
    }
}
