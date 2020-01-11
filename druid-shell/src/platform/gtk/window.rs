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
use std::ffi::c_void;
use std::ffi::OsString;
use std::os::raw::{c_int, c_uint};
use std::ptr;
use std::slice;
use std::sync::{Arc, Mutex, Weak};

use gdk::{EventKey, EventMask, ModifierType, ScrollDirection, WindowExt};
use gio::ApplicationExt;
use gtk::prelude::*;
use gtk::{AccelGroup, ApplicationWindow};

use crate::kurbo::{Point, Size, Vec2};
use crate::piet::{Piet, RenderContext};

use super::dialog;
use super::menu::Menu;
use super::runloop::with_application;
use super::util::assert_main_thread;

use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::keyboard;
use crate::mouse::{Cursor, MouseButton, MouseEvent};
use crate::window::{Text, TimerToken, WinCtx, WinHandler};
use crate::Error;

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
pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    size: Size,
}

#[derive(Clone)]
pub struct IdleHandle {
    idle_queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
    state: Weak<WindowState>,
}

pub(crate) struct WindowState {
    window: ApplicationWindow,
    pub(crate) handler: RefCell<Box<dyn WinHandler>>,
    idle_queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
    current_keyval: RefCell<Option<u32>>,
}

pub(crate) struct WinCtxImpl<'a> {
    handle: &'a WindowHandle,
    text: Text<'static>,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(500.0, 400.0),
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        assert_main_thread();

        let handler = self
            .handler
            .expect("Tried to build a window without setting the handler");

        let window = with_application(|app| ApplicationWindow::new(&app));

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

        let win_state = Arc::new(WindowState {
            window,
            handler: RefCell::new(handler),
            idle_queue: Arc::new(Mutex::new(vec![])),
            current_keyval: RefCell::new(None),
        });

        with_application(|app| {
            app.connect_shutdown(clone!(win_state => move |_| {
                // this ties a clone of Arc<WindowState> to the ApplicationWindow to keep it alive
                // when the ApplicationWindow is destroyed, the last Arc is dropped
                // and any Weak<WindowState> will be None on upgrade()
                let _ = &win_state;
            }))
        });

        let handle = WindowHandle {
            state: Arc::downgrade(&win_state),
        };

        if let Some(menu) = self.menu {
            let menu = menu.into_gtk_menubar(&handle, &accel_group);
            vbox.pack_start(&menu, false, false, 0);
        }

        let drawing_area = gtk::DrawingArea::new();

        drawing_area.set_events(
            EventMask::EXPOSURE_MASK
                | EventMask::POINTER_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::BUTTON_RELEASE_MASK
                | EventMask::KEY_PRESS_MASK
                | EventMask::ENTER_NOTIFY_MASK
                | EventMask::KEY_RELEASE_MASK
                | EventMask::SCROLL_MASK,
        );

        drawing_area.set_can_focus(true);
        drawing_area.grab_focus();

        drawing_area.connect_enter_notify_event(|widget, _| {
            widget.grab_focus();

            Inhibit(true)
        });

        let last_size = Cell::new((0, 0));

        drawing_area.connect_draw(clone!(handle => move |widget, context| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                let extents = context.clip_extents();
                let dpi_scale = state.window.get_window()
                    .map(|w| w.get_display().get_default_screen().get_resolution())
                    .unwrap_or(96.0) / 96.0;
                let size = (
                    ((extents.2 - extents.0) * dpi_scale) as u32,
                    ((extents.3 - extents.1) * dpi_scale) as u32,
                );

                if last_size.get() != size {
                    last_size.set(size);
                    state.handler.borrow_mut().size(size.0, size.1, &mut ctx);
                }

                // For some reason piet needs a mutable context, so give it one I guess.
                let mut context = context.clone();
                let mut piet_context = Piet::new(&mut context);

                if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                    let anim = handler_borrow
                        .paint(&mut piet_context, &mut ctx);
                    if let Err(e) = piet_context.finish() {
                        eprintln!("piet error on render: {:?}", e);
                    }

                    if anim {
                        widget.queue_draw();
                    }
                }

            }

            Inhibit(false)
        }));

        drawing_area.connect_button_press_event(clone!(handle => move |_widget, button| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                state.handler.borrow_mut().mouse_down(
                    &MouseEvent {
                        pos: Point::from(button.get_position()),
                        count: get_mouse_click_count(button.get_event_type()),
                        mods: get_modifiers(button.get_state()),
                        button: get_mouse_button(button.get_button()),
                    },
                    &mut ctx,
                );
            }

            Inhibit(true)
        }));

        drawing_area.connect_button_release_event(clone!(handle => move |_widget, button| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                state.handler.borrow_mut().mouse_up(
                    &MouseEvent {
                        pos: Point::from(button.get_position()),
                        mods: get_modifiers(button.get_state()),
                        count: 0,
                        button: get_mouse_button(button.get_button()),
                    },
                    &mut ctx,
                );
            }

            Inhibit(true)
        }));

        drawing_area.connect_motion_notify_event(clone!(handle=>move |_widget, motion| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                let pos = Point::from(motion.get_position());
                let mouse_event = MouseEvent {
                    pos,
                    mods: get_modifiers(motion.get_state()),
                    count: 0,
                    button: get_mouse_button_from_modifiers(motion.get_state()),
                };

                state
                    .handler
                    .borrow_mut()
                    .mouse_move(&mouse_event, &mut ctx);
            }

            Inhibit(true)
        }));

        drawing_area.connect_scroll_event(clone!(handle => move |_widget, scroll| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                let modifiers = get_modifiers(scroll.get_state());

                // The magic "120"s are from Microsoft's documentation for WM_MOUSEWHEEL.
                // They claim that one "tick" on a scroll wheel should be 120 units.
                let mut handler = state.handler.borrow_mut();
                match scroll.get_direction() {
                    ScrollDirection::Up => {
                        handler.wheel(Vec2::from((0.0, -120.0)), modifiers, &mut ctx);
                    }
                    ScrollDirection::Down => {
                        handler.wheel(Vec2::from((0.0, 120.0)), modifiers, &mut ctx);
                    }
                    ScrollDirection::Left => {
                        handler.wheel(Vec2::from((-120.0, 0.0)), modifiers, &mut ctx);
                    }
                    ScrollDirection::Right => {
                        handler.wheel(Vec2::from((120.0, 0.0)), modifiers, &mut ctx);
                    }
                    ScrollDirection::Smooth => {
                        // TODO: support smooth scrolling via scroll.get_delta and get_is_stop
                        eprintln!(
                            "Warning: somehow the Druid widget got a smooth scroll event"
                        );
                    }
                    e => {
                        eprintln!(
                            "Warning: the Druid widget got some whacky scroll direction {:?}",
                            e
                        );
                    }
                }
            }

            Inhibit(true)
        }));

        drawing_area.connect_key_press_event(clone!(handle => move |_widget, key| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                let mut current_keyval = state.current_keyval.borrow_mut();
                let repeat = *current_keyval == Some(key.get_keyval());

                *current_keyval = Some(key.get_keyval());

                let key_event = make_key_event(key, repeat);
                state.handler.borrow_mut().key_down(key_event, &mut ctx);
            }

            Inhibit(true)
        }));

        drawing_area.connect_key_release_event(clone!(handle => move |_widget, key| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);

                *(state.current_keyval.borrow_mut()) = None;

                let key_event = make_key_event(key, false);
                state.handler.borrow_mut().key_up(key_event, &mut ctx);
            }

            Inhibit(true)
        }));

        drawing_area.connect_destroy(clone!(handle => move |_widget| {
            if let Some(state) = handle.state.upgrade() {
                let mut ctx = WinCtxImpl::from(&handle);
                state.handler.borrow_mut().destroy(&mut ctx);
            }
        }));

        vbox.pack_end(&drawing_area, true, true, 0);

        win_state
            .handler
            .borrow_mut()
            .connect(&handle.clone().into());

        let mut ctx = WinCtxImpl::from(&handle);
        win_state.handler.borrow_mut().connected(&mut ctx);

        Ok(handle)
    }
}

impl WindowHandle {
    pub fn show(&self) {
        if let Some(state) = self.state.upgrade() {
            state.window.show_all();
        }
    }

    /// Close the window.
    pub fn close(&self) {
        if let Some(state) = self.state.upgrade() {
            with_application(|app| {
                app.remove_window(&state.window);
            });
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
    pub fn add_idle<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        let mut queue = self.idle_queue.lock().unwrap();
        if let Some(state) = self.state.upgrade() {
            if queue.is_empty() {
                queue.push(Box::new(callback));
                gdk::threads_add_idle(move || run_idle(&state));
            } else {
                queue.push(Box::new(callback));
            }
        }
    }
}

fn run_idle(state: &Arc<WindowState>) -> bool {
    assert_main_thread();
    let mut handler = state.handler.borrow_mut();
    let handler_as_any = handler.as_any();

    let queue: Vec<_> = std::mem::replace(&mut state.idle_queue.lock().unwrap(), Vec::new());

    for callback in queue {
        callback.call(handler_as_any);
    }
    false
}

impl<'a> WinCtx<'a> for WinCtxImpl<'a> {
    fn invalidate(&mut self) {
        self.handle.invalidate();
    }

    fn text_factory(&mut self) -> &mut Text<'a> {
        &mut self.text
    }

    fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(gdk_window) = self
            .handle
            .state
            .upgrade()
            .and_then(|s| s.window.get_window())
        {
            let cursor = make_gdk_cursor(cursor, &gdk_window);
            gdk_window.set_cursor(cursor.as_ref());
        }
    }

    fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        self.handle
            .file_dialog(FileDialogType::Open, options)
            .ok()
            .map(|s| FileInfo { path: s.into() })
    }

    fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        self.handle
            .file_dialog(FileDialogType::Save, options)
            .ok()
            .map(|s| FileInfo { path: s.into() })
    }

    fn request_timer(&mut self, deadline: std::time::Instant) -> TimerToken {
        let interval = time_interval_from_deadline(deadline);
        let token = next_timer_id();

        let handle = self.handle.clone();

        gdk::threads_add_timeout(interval, move || {
            if let Some(state) = handle.state.upgrade() {
                if let Ok(mut handler_borrow) = state.handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl::from(&handle);
                    handler_borrow.timer(TimerToken::new(token), &mut ctx);
                    return false;
                }
            }
            true
        });

        TimerToken::new(token)
    }
}

impl<'a> From<&'a WindowHandle> for WinCtxImpl<'a> {
    fn from(handle: &'a WindowHandle) -> Self {
        WinCtxImpl {
            handle,
            text: Text::new(),
        }
    }
}

fn time_interval_from_deadline(deadline: std::time::Instant) -> u32 {
    let now = std::time::Instant::now();
    if now >= deadline {
        0
    } else {
        (deadline - now).as_millis() as u32
    }
}

fn next_timer_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TIMER_ID: AtomicUsize = AtomicUsize::new(1);
    TIMER_ID.fetch_add(1, Ordering::Relaxed)
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

fn get_mouse_button(button: u32) -> MouseButton {
    match button {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        4 => MouseButton::X1,
        5 => MouseButton::X2,
        _ => MouseButton::Left,
    }
}

fn get_mouse_button_from_modifiers(modifiers: gdk::ModifierType) -> MouseButton {
    match modifiers {
        modifiers if modifiers.contains(ModifierType::BUTTON1_MASK) => MouseButton::Left,
        modifiers if modifiers.contains(ModifierType::BUTTON2_MASK) => MouseButton::Middle,
        modifiers if modifiers.contains(ModifierType::BUTTON3_MASK) => MouseButton::Right,
        modifiers if modifiers.contains(ModifierType::BUTTON4_MASK) => MouseButton::X1,
        modifiers if modifiers.contains(ModifierType::BUTTON5_MASK) => MouseButton::X2,
        _ => {
            //FIXME: what about when no modifiers match?
            MouseButton::Left
        }
    }
}

fn get_mouse_click_count(event_type: gdk::EventType) -> u32 {
    match event_type {
        gdk::EventType::ButtonPress => 1,
        gdk::EventType::DoubleButtonPress => 2,
        gdk::EventType::TripleButtonPress => 3,
        _ => 0,
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
