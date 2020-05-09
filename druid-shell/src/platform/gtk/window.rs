// Copyright 2019 The xi-editor Authors.
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

use gdk::{EventKey, EventMask, ModifierType, ScrollDirection, WindowExt};
use gio::ApplicationExt;
use gtk::prelude::*;
use gtk::{AccelGroup, ApplicationWindow, DrawingArea};

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::{Piet, RenderContext};

use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::keyboard;
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
use crate::window::{IdleToken, Text, TimerToken, WinHandler};
use crate::Error;

use super::application::Application;
use super::dialog;
use super::menu::Menu;
use super::util;

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
    drawing_area: DrawingArea,
    pub(crate) handler: RefCell<Box<dyn WinHandler>>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    current_keyval: RefCell<Option<u32>>,
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(500.0, 400.0),
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

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        let handler = self
            .handler
            .expect("Tried to build a window without setting the handler");

        let window = ApplicationWindow::new(self.app.gtk_app());

        window.set_title(&self.title);
        window.set_resizable(self.resizable);
        window.set_decorated(self.show_titlebar);

        let dpi_scale = window
            .get_display()
            .map(|c| c.get_default_screen().get_resolution() as f64)
            .unwrap_or(96.0)
            / 96.0;

        window.set_default_size(
            (self.size.width * dpi_scale) as i32,
            (self.size.height * dpi_scale) as i32,
        );

        let accel_group = AccelGroup::new();
        window.add_accel_group(&accel_group);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&vbox);
        let drawing_area = gtk::DrawingArea::new();

        let win_state = Arc::new(WindowState {
            window,
            drawing_area,
            handler: RefCell::new(handler),
            idle_queue: Arc::new(Mutex::new(vec![])),
            current_keyval: RefCell::new(None),
        });

        self.app
            .gtk_app()
            .connect_shutdown(clone!(win_state => move |_| {
                // this ties a clone of Arc<WindowState> to the ApplicationWindow to keep it alive
                // when the ApplicationWindow is destroyed, the last Arc is dropped
                // and any Weak<WindowState> will be None on upgrade()
                let _ = &win_state;
            }));

        let handle = WindowHandle {
            state: Arc::downgrade(&win_state),
        };

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

        if let Some(min_size) = self.min_size {
            win_state.drawing_area.set_size_request(
                (min_size.width * dpi_scale) as i32,
                (min_size.height * dpi_scale) as i32,
            );
        }

        let last_size = Cell::new((0, 0));

        win_state.drawing_area.connect_draw(clone!(handle => move |widget, context| {
            if let Some(state) = handle.state.upgrade() {

                let extents = widget.get_allocation();
                let dpi_scale = state.window.get_window()
                    .map(|w| w.get_display().get_default_screen().get_resolution())
                    .unwrap_or(96.0) / 96.0;
                let size = ((extents.width as f64 * dpi_scale) as u32, (extents.height as f64 * dpi_scale) as u32);

                if last_size.get() != size {
                    if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                        last_size.set(size);
                        handler_borrow.size(size.0, size.1);
                    } else {
                        log::warn!("Resizing was skipped because the handler was already borrowed");
                    }
                }

                // For some reason piet needs a mutable context, so give it one I guess.
                let mut context = context.clone();
                let (x0, y0, x1, y1) = context.clip_extents();
                let mut piet_context = Piet::new(&mut context);

                if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                    let invalid_rect = Rect::new(x0 * dpi_scale, y0 * dpi_scale, x1 * dpi_scale, y1 * dpi_scale);
                    let anim = handler_borrow
                        .paint(&mut piet_context, invalid_rect);
                    if let Err(e) = piet_context.finish() {
                        eprintln!("piet error on render: {:?}", e);
                    }

                    if anim {
                        widget.queue_draw();
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
                        let button_state = event.get_state();
                        handler.mouse_down(
                            &MouseEvent {
                                pos: Point::from(event.get_position()),
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
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_button_release_event(clone!(handle => move |_widget, event| {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    if let Some(button) = get_mouse_button(event.get_button()) {
                        let button_state = event.get_state();
                        handler.mouse_up(
                            &MouseEvent {
                                pos: Point::from(event.get_position()),
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
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_motion_notify_event(clone!(handle => move |_widget, motion| {
            if let Some(state) = handle.state.upgrade() {
                let motion_state = motion.get_state();
                let mouse_event = MouseEvent {
                    pos: Point::from(motion.get_position()),
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
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_leave_notify_event(clone!(handle => move |_widget, crossing| {
            if let Some(state) = handle.state.upgrade() {
                let crossing_state = crossing.get_state();
                let mouse_event = MouseEvent {
                    pos: Point::from(crossing.get_position()),
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
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_scroll_event(clone!(handle => move |_widget, scroll| {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler) = state.handler.try_borrow_mut() {

                    let mods = get_modifiers(scroll.get_state());

                    // The magic "120"s are from Microsoft's documentation for WM_MOUSEWHEEL.
                    // They claim that one "tick" on a scroll wheel should be 120 units.
                    let wheel_delta = match scroll.get_direction() {
                        ScrollDirection::Up if mods.shift => Some(Vec2::new(-120.0, 0.0)),
                        ScrollDirection::Up => Some(Vec2::new(0.0, -120.0)),
                        ScrollDirection::Down if mods.shift => Some(Vec2::new(120.0, 0.0)),
                        ScrollDirection::Down => Some(Vec2::new(0.0, 120.0)),
                        ScrollDirection::Left => Some(Vec2::new(-120.0, 0.0)),
                        ScrollDirection::Right => Some(Vec2::new(120.0, 0.0)),
                        ScrollDirection::Smooth => {
                            //TODO: Look at how gtk's scroll containers implements it
                            let (mut delta_x, mut delta_y) = scroll.get_delta();
                            delta_x *= 120.;
                            delta_y *= 120.;
                            if mods.shift {
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
                            pos: Point::from(scroll.get_position()),
                            buttons: get_mouse_buttons_from_modifiers(scroll.get_state()),
                            mods,
                            count: 0,
                            focus: false,
                            button: MouseButton::None,
                            wheel_delta
                        };

                        handler.wheel(&mouse_event);
                    }
                } else {
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_key_press_event(clone!(handle => move |_widget, key| {
            if let Some(state) = handle.state.upgrade() {

                let mut current_keyval = state.current_keyval.borrow_mut();
                let repeat = *current_keyval == Some(key.get_keyval());

                *current_keyval = Some(key.get_keyval());

                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    handler.key_down(make_key_event(key, repeat));
                } else {
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
        }));

        win_state.drawing_area.connect_key_release_event(clone!(handle => move |_widget, key| {
            if let Some(state) = handle.state.upgrade() {

                *(state.current_keyval.borrow_mut()) = None;

                if let Ok(mut handler) = state.handler.try_borrow_mut() {
                    handler.key_up(make_key_event(key, false));
                } else {
                    log::info!("GTK event was dropped because the handler was already borrowed");
                }
            }

            Inhibit(true)
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

        win_state
            .handler
            .borrow_mut()
            .connect(&handle.clone().into());

        Ok(handle)
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

    /// Close the window.
    pub fn close(&self) {
        if let Some(state) = self.state.upgrade() {
            state.window.close();
        }
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        //FIXME: implementation goes here
        log::warn!("bring_to_front_and_focus not yet implemented for gtk");
    }

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        if let Some(state) = self.state.upgrade() {
            state.window.queue_draw();
        }
    }

    /// Request invalidation of one rectangle, which is given relative to the drawing area.
    pub fn invalidate_rect(&self, rect: Rect) {
        let dpi_scale = self.get_dpi() as f64 / 96.0;
        let rect = Rect::from_origin_size(
            (rect.x0 * dpi_scale, rect.y0 * dpi_scale),
            rect.size() * dpi_scale,
        );

        // GTK+ takes rects with integer coordinates, and non-negative width/height.
        let r = rect.abs().expand();

        if let Some(state) = self.state.upgrade() {
            let origin = state.drawing_area.get_allocation();
            state.window.queue_draw_area(
                r.x0 as i32 + origin.x,
                r.y0 as i32 + origin.y,
                r.width() as i32,
                r.height() as i32,
            );
        }
    }

    pub fn text(&self) -> Text {
        Text::new()
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

        gdk::threads_add_timeout(interval, move || {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                    handler_borrow.timer(token);
                    return false;
                }
            }
            true
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

    /// Get the dpi of the window.
    ///
    /// TODO: we want to migrate this from dpi (with 96 as nominal) to a scale
    /// factor (with 1 as nominal).
    pub fn get_dpi(&self) -> f32 {
        self.state
            .upgrade()
            .and_then(|s| s.window.get_window())
            .map(|w| w.get_display().get_default_screen().get_resolution() as f32)
            .unwrap_or(96.0)
    }

    // TODO: the following methods are cut'n'paste code. A good way to DRY
    // would be to have a platform-independent trait with these as methods with
    // default implementations.

    /// Convert a dimension in px units to physical pixels (rounding).
    pub fn px_to_pixels(&self, x: f32) -> i32 {
        (x * self.get_dpi() * (1.0 / 96.0)).round() as i32
    }

    /// Convert a point in px units to physical pixels (rounding).
    pub fn px_to_pixels_xy(&self, x: f32, y: f32) -> (i32, i32) {
        let scale = self.get_dpi() * (1.0 / 96.0);
        ((x * scale).round() as i32, (y * scale).round() as i32)
    }

    /// Convert a dimension in physical pixels to px units.
    pub fn pixels_to_px<T: Into<f64>>(&self, x: T) -> f32 {
        (x.into() as f32) * 96.0 / self.get_dpi()
    }

    /// Convert a point in physical pixels to px units.
    pub fn pixels_to_px_xy<T: Into<f64>>(&self, x: T, y: T) -> (f32, f32) {
        let scale = 96.0 / self.get_dpi();
        ((x.into() as f32) * scale, (y.into() as f32) * scale)
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
    ) -> Result<OsString, Error> {
        if let Some(state) = self.state.upgrade() {
            dialog::get_file_dialog_path(state.window.upcast_ref(), ty, options)
        } else {
            Err(Error::Other(
                "Cannot upgrade state from weak pointer to arc",
            ))
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
    let mut handler = state.handler.borrow_mut();

    let queue: Vec<_> = std::mem::replace(&mut state.idle_queue.lock().unwrap(), Vec::new());

    for item in queue {
        match item {
            IdleKind::Callback(it) => it.call(handler.as_any()),
            IdleKind::Token(it) => handler.idle(it),
        }
    }
    glib::source::Continue(false)
}

fn make_gdk_cursor(cursor: &Cursor, gdk_window: &gdk::Window) -> Option<gdk::Cursor> {
    gdk::Cursor::new_from_name(
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

fn get_modifiers(modifiers: gdk::ModifierType) -> keyboard::KeyModifiers {
    keyboard::KeyModifiers {
        shift: modifiers.contains(ModifierType::SHIFT_MASK),
        alt: modifiers.contains(ModifierType::MOD1_MASK),
        ctrl: modifiers.contains(ModifierType::CONTROL_MASK),
        meta: modifiers.contains(ModifierType::META_MASK),
    }
}

fn make_key_event(key: &EventKey, repeat: bool) -> keyboard::KeyEvent {
    let keyval = key.get_keyval();
    let hardware_keycode = key.get_hardware_keycode();

    let keycode = hardware_keycode_to_keyval(hardware_keycode).unwrap_or(keyval);

    let text = gdk::keyval_to_unicode(keyval);

    keyboard::KeyEvent::new(keycode, repeat, get_modifiers(key.get_state()), text, text)
}

/// Map a hardware keycode to a keyval by performing a lookup in the keymap and finding the
/// keyval with the lowest group and level
fn hardware_keycode_to_keyval(keycode: u16) -> Option<u32> {
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
                    Some(keyvals_slice[i])
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
