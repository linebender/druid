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
use std::convert::{TryFrom, TryInto};
use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};
use std::panic::Location;
use std::ptr;
use std::slice;
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use anyhow::anyhow;
use cairo::Surface;
use gdk::{EventKey, EventMask, ModifierType, ScrollDirection, WindowExt, WindowTypeHint};
use gio::ApplicationExt;
use gtk::prelude::*;
use gtk::{AccelGroup, ApplicationWindow, DrawingArea, SettingsExt};

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::{Piet, PietText, RenderContext};

use crate::common_util::{ClickCounter, IdleCallback};
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::error::Error as ShellError;
use crate::keyboard::{KbKey, KeyEvent, KeyState, Modifiers};
use crate::mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent};
use crate::piet::ImageFormat;
use crate::region::Region;
use crate::scale::{Scalable, Scale, ScaledArea};
use crate::window;
use crate::window::{FileDialogToken, IdleToken, TimerToken, WinHandler, WindowLevel};

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
    // Ensure that we don't implement Send, because it isn't actually safe to send the WindowState.
    marker: std::marker::PhantomData<*const ()>,
}

/// Operations that we defer in order to avoid re-entrancy. See the documentation in the windows
/// backend for more details.
enum DeferredOp {
    SaveAs(FileDialogOptions, FileDialogToken),
    Open(FileDialogOptions, FileDialogToken),
    ContextMenu(Menu, WindowHandle),
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

// We use RefCells for interior mutability, but we try to structure things so that double-borrows
// are impossible. See the documentation on crate::platform::x11::window::Window for more details,
// since the idea there is basically the same.
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
    surface_size: Cell<(i32, i32)>,
    // The invalid region, in display points.
    invalid: RefCell<Region>,
    pub(crate) handler: RefCell<Box<dyn WinHandler>>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    current_keycode: Cell<Option<u16>>,
    click_counter: ClickCounter,
    deferred_queue: RefCell<Vec<DeferredOp>>,
}

#[derive(Clone, PartialEq)]
pub struct CustomCursor(gdk::Cursor);

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
            surface_size: Cell::new((0, 0)),
            invalid: RefCell::new(Region::EMPTY),
            handler: RefCell::new(handler),
            idle_queue: Arc::new(Mutex::new(vec![])),
            current_keycode: Cell::new(None),
            click_counter: ClickCounter::default(),
            deferred_queue: RefCell::new(Vec::new()),
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
            marker: std::marker::PhantomData,
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
                | EventMask::SMOOTH_SCROLL_MASK
                | EventMask::FOCUS_CHANGE_MASK,
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
                        state.with_handler(|h| h.scale(scale));
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
                    state.with_handler(|h| h.size(size_dp));
                    state.invalidate_rect(size_dp.to_rect());
                }

                state.with_handler(|h| h.prepare_paint());

                let invalid = match state.invalid.try_borrow_mut() {
                    Ok(mut invalid) => std::mem::replace(&mut *invalid, Region::EMPTY),
                    Err(_) => {
                        log::error!("invalid region borrowed while drawing");
                        Region::EMPTY
                    }
                };

                if let Ok(Some(surface)) = state.surface.try_borrow().as_ref().map(|s| s.as_ref()) {
                    // Note that we're borrowing the surface while calling the handler. This is ok,
                    // because we don't return control to the system or re-borrow the surface from
                    // any code that the client can call.
                    state.with_handler_and_dont_check_the_other_borrows(|handler| {
                        let surface_context = cairo::Context::new(surface);

                        // Clip to the invalid region, in order that our surface doesn't get
                        // messed up if there's any painting outside them.
                        for rect in invalid.rects() {
                            let rect = rect.to_px(scale);
                            surface_context.rectangle(rect.x0, rect.y0, rect.width(), rect.height());
                        }
                        surface_context.clip();

                        surface_context.scale(scale.x(), scale.y());
                        let mut piet_context = Piet::new(&surface_context);
                        handler.paint(&mut piet_context, &invalid);
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
                    });
                } else {
                    log::warn!("Drawing was skipped because there was no surface");
                }
            }

            Inhibit(false)
        }));

        win_state.drawing_area.connect_button_press_event(clone!(handle => move |_widget, event| {
            if let Some(state) = handle.state.upgrade() {
                state.with_handler(|handler| {
                    if let Some(button) = get_mouse_button(event.get_button()) {
                        let scale = state.scale.get();
                        let button_state = event.get_state();
                        let gtk_count = get_mouse_click_count(event.get_event_type());
                        let pos: Point =  event.get_position().into();
                        let count = if gtk_count == 1 {
                            let settings = state.drawing_area.get_settings().unwrap();
                            let thresh_dist = settings.get_property_gtk_double_click_distance();
                            state.click_counter.set_distance(thresh_dist.into());
                            if let Ok(ms) = settings.get_property_gtk_double_click_time().try_into() {
                                state.click_counter.set_interval_ms(ms);
                            }
                            state.click_counter.count_for_click(pos)
                        } else {
                            0
                        };
                        if gtk_count == 0 || gtk_count == 1 {
                            handler.mouse_down(
                                &MouseEvent {
                                    pos: pos.to_dp(scale),
                                    buttons: get_mouse_buttons_from_modifiers(button_state).with(button),
                                    mods: get_modifiers(button_state),
                                    count,
                                    focus: false,
                                    button,
                                    wheel_delta: Vec2::ZERO
                                },
                            );
                        }
                    }
                });
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_button_release_event(clone!(handle => move |_widget, event| {
            if let Some(state) = handle.state.upgrade() {
                state.with_handler(|handler| {
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
                });
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_motion_notify_event(
            clone!(handle => move |_widget, motion| {
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

                    state.with_handler(|h| h.mouse_move(&mouse_event));
                }

                Inhibit(true)
            }),
        );

        win_state.drawing_area.connect_leave_notify_event(
            clone!(handle => move |_widget, crossing| {
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

                    state.with_handler(|h| h.mouse_move(&mouse_event));
                }

                Inhibit(true)
            }),
        );

        win_state
            .drawing_area
            .connect_scroll_event(clone!(handle => move |_widget, scroll| {
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

                        state.with_handler(|h| h.wheel(&mouse_event));
                    }
                }

                Inhibit(true)
            }));

        win_state
            .drawing_area
            .connect_key_press_event(clone!(handle => move |_widget, key| {
                if let Some(state) = handle.state.upgrade() {

                    let hw_keycode = key.get_hardware_keycode();
                    let repeat = state.current_keycode.get() == Some(hw_keycode);

                    state.current_keycode.set(Some(hw_keycode));

                    state.with_handler(|h|
                        h.key_down(make_key_event(key, repeat, KeyState::Down))
                    );
                }

                Inhibit(true)
            }));

        win_state
            .drawing_area
            .connect_key_release_event(clone!(handle => move |_widget, key| {
                if let Some(state) = handle.state.upgrade() {

                    if state.current_keycode.get() == Some(key.get_hardware_keycode()) {
                        state.current_keycode.set(None);
                    }


                    state.with_handler(|h|
                        h.key_up(make_key_event(key, false, KeyState::Up))
                    );
                }

                Inhibit(true)
            }));

        win_state
            .drawing_area
            .connect_focus_in_event(clone!(handle => move |_widget, _event| {
                if let Some(state) = handle.state.upgrade() {
                    state.with_handler(|h| h.got_focus());
                }
                Inhibit(true)
            }));

        win_state
            .drawing_area
            .connect_focus_out_event(clone!(handle => move |_widget, _event| {
                if let Some(state) = handle.state.upgrade() {
                    state.with_handler(|h| h.lost_focus());
                }
                Inhibit(true)
            }));

        win_state
            .window
            .connect_delete_event(clone!(handle => move |_widget, _ev| {
                if let Some(state) = handle.state.upgrade() {
                    state.with_handler(|h| h.request_close());
                    Inhibit(!state.closing.get())
                } else {
                    Inhibit(false)
                }
            }));

        win_state
            .drawing_area
            .connect_destroy(clone!(handle => move |_widget| {
                if let Some(state) = handle.state.upgrade() {
                    state.with_handler(|h| h.destroy());
                }
            }));

        vbox.pack_end(&win_state.drawing_area, true, true, 0);
        win_state.drawing_area.realize();
        win_state
            .drawing_area
            .get_window()
            .expect("realize didn't create window")
            .set_event_compression(false);

        let size = self.size;
        win_state.with_handler(|h| {
            h.connect(&handle.clone().into());
            h.scale(scale);
            h.size(size);
        });

        Ok(handle)
    }
}

impl WindowState {
    #[track_caller]
    fn with_handler<T, F: FnOnce(&mut dyn WinHandler) -> T>(&self, f: F) -> Option<T> {
        if self.invalid.try_borrow_mut().is_err() || self.surface.try_borrow_mut().is_err() {
            log::error!("other RefCells were borrowed when calling into the handler");
            return None;
        }

        let ret = self.with_handler_and_dont_check_the_other_borrows(f);

        self.run_deferred();
        ret
    }

    #[track_caller]
    fn with_handler_and_dont_check_the_other_borrows<T, F: FnOnce(&mut dyn WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        match self.handler.try_borrow_mut() {
            Ok(mut h) => Some(f(&mut **h)),
            Err(_) => {
                log::error!("failed to borrow WinHandler at {}", Location::caller());
                None
            }
        }
    }

    fn resize_surface(&self, width: i32, height: i32) -> Result<(), anyhow::Error> {
        fn next_size(x: i32) -> i32 {
            // We round up to the nearest multiple of `accuracy`, which is between x/2 and x/4.
            // Don't bother rounding to anything smaller than 32 = 2^(7-1).
            let accuracy = 1 << ((32 - x.leading_zeros()).max(7) - 2);
            let mask = accuracy - 1;
            (x + mask) & !mask
        }

        let mut surface = self.surface.borrow_mut();
        let mut cur_size = self.surface_size.get();
        let (width, height) = (next_size(width), next_size(height));
        if surface.is_none() || cur_size != (width, height) {
            cur_size = (width, height);
            self.surface_size.set(cur_size);
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

    /// Pushes a deferred op onto the queue.
    fn defer(&self, op: DeferredOp) {
        self.deferred_queue.borrow_mut().push(op);
    }

    fn run_deferred(&self) {
        let queue = self.deferred_queue.replace(Vec::new());
        for op in queue {
            match op {
                DeferredOp::Open(options, token) => {
                    let file_info = dialog::get_file_dialog_path(
                        self.window.upcast_ref(),
                        FileDialogType::Open,
                        options,
                    )
                    .ok()
                    .map(|s| FileInfo { path: s.into() });
                    self.with_handler(|h| h.open_file(token, file_info));
                }
                DeferredOp::SaveAs(options, token) => {
                    let file_info = dialog::get_file_dialog_path(
                        self.window.upcast_ref(),
                        FileDialogType::Save,
                        options,
                    )
                    .ok()
                    .map(|s| FileInfo { path: s.into() });
                    self.with_handler(|h| h.save_as(token, file_info));
                }
                DeferredOp::ContextMenu(menu, handle) => {
                    let accel_group = AccelGroup::new();
                    self.window.add_accel_group(&accel_group);

                    let menu = menu.into_gtk_menu(&handle, &accel_group);
                    menu.set_property_attach_widget(Some(&self.window));
                    menu.show_all();
                    menu.popup_easy(3, gtk::get_current_event_time());
                }
            }
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

        if let Some(state) = self.state.upgrade() {
            glib::timeout_add(interval, move || {
                if state.with_handler(|h| h.timer(token)).is_some() {
                    return glib::Continue(false);
                }
                glib::Continue(true)
            });
        }
        token
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(gdk_window) = self.state.upgrade().and_then(|s| s.window.get_window()) {
            let cursor = make_gdk_cursor(cursor, &gdk_window);
            gdk_window.set_cursor(cursor.as_ref());
        }
    }

    pub fn make_cursor(&self, desc: &CursorDesc) -> Option<Cursor> {
        if let Some(state) = self.state.upgrade() {
            if let Some(gdk_window) = state.window.get_window() {
                // TODO: gtk::Pixbuf expects unpremultiplied alpha. We should convert.
                let has_alpha = !matches!(desc.image.format(), ImageFormat::Rgb);
                let bytes_per_pixel = desc.image.format().bytes_per_pixel();
                let pixbuf = gdk_pixbuf::Pixbuf::from_mut_slice(
                    desc.image.raw_pixels().to_owned(),
                    gdk_pixbuf::Colorspace::Rgb,
                    has_alpha,
                    // bits_per_sample
                    8,
                    desc.image.width() as i32,
                    desc.image.height() as i32,
                    // row stride (in bytes)
                    (desc.image.width() * bytes_per_pixel) as i32,
                );
                let c = gdk::Cursor::from_pixbuf(
                    &gdk_window.get_display(),
                    &pixbuf,
                    desc.hot.x.round() as i32,
                    desc.hot.y.round() as i32,
                );
                Some(Cursor::Custom(CustomCursor(c)))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn open_file(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        if let Some(state) = self.state.upgrade() {
            let tok = FileDialogToken::next();
            state.defer(DeferredOp::Open(options, tok));
            Some(tok)
        } else {
            None
        }
    }

    pub fn save_as(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        if let Some(state) = self.state.upgrade() {
            let tok = FileDialogToken::next();
            state.defer(DeferredOp::SaveAs(options, tok));
            Some(tok)
        } else {
            None
        }
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
            state.defer(DeferredOp::ContextMenu(menu, self.clone()));
        }
    }

    pub fn set_title(&self, title: impl Into<String>) {
        if let Some(state) = self.state.upgrade() {
            state.window.set_title(&(title.into()));
        }
    }
}

// WindowState needs to be Send + Sync so it can be passed into glib closures.
// TODO: can we localize the unsafety more? Glib's idle loop always runs on the main thread,
// and we always construct the WindowState on the main thread, so it should be ok (and also
// WindowState isn't a public type).
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
    let result = state.with_handler(|handler| {
        let queue: Vec<_> = std::mem::replace(&mut state.idle_queue.lock().unwrap(), Vec::new());

        for item in queue {
            match item {
                IdleKind::Callback(it) => it.call(handler.as_any()),
                IdleKind::Token(it) => handler.idle(it),
            }
        }
    });

    if result.is_none() {
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
    if let Cursor::Custom(custom) = cursor {
        Some(custom.0.clone())
    } else {
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
                Cursor::Custom(_) => unreachable!(),
            },
        )
    }
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
