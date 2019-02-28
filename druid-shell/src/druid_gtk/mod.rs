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

//! GTK-based platform support

pub mod application;
pub mod dialog;
pub mod menu;
pub mod util;
pub mod win_main;

use std::any::Any;
use std::cell::Cell;
use std::ffi::OsString;
use std::sync::{Arc, Mutex, Weak};

use gdk::EventMask;
use gtk::ApplicationWindow;
use gtk::Inhibit;

use piet::RenderContext;
use piet_common::Piet;

use platform::dialog::{FileDialogOptions, FileDialogType};
use platform::menu::Menu;
use window::{self, Cursor, WinHandler};
use Error;

use util::assert_main_thread;
use win_main::with_application;

#[derive(Clone, Default)]
pub struct WindowHandle {
    window: Option<ApplicationWindow>,
}

/// Builder abstraction for creating new windows
pub struct WindowBuilder {
    handler: Option<Box<WinHandler>>,
    title: String,
    cursor: Cursor,
}

#[derive(Clone)]
pub struct IdleHandle {
    queue: Option<u32>, // TODO: implement this properly
    idle_queue: Weak<Mutex<Vec<Box<IdleCallback>>>>,
}

// TODO: move this out of platform-dependent section.
trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &Any);
}

impl<F: FnOnce(&Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &Any) {
        (*self)(a)
    }
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            cursor: Cursor::Arrow,
        }
    }

    pub fn set_handler(&mut self, handler: Box<WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        // TODO
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        use gtk::{ContainerExt, GtkWindowExt, WidgetExt};
        assert_main_thread();

        let handler = self
            .handler
            .expect("Tried to build a window without setting the handler");

        let handler = Arc::new(handler);

        let mut window = with_application(|app| ApplicationWindow::new(&app));

        window.set_title(&self.title);
        // TODO(bobtwinkles): enable this when I figure out how to set the cursor on application windows
        // window.set_cursor(gdk::Cursor::new_from_nane(
        //     &window.get_display(),
        //     match self.cursor {
        //         Arrow => "default",
        //         IBeam => "text",
        //     },
        // ));

        let drawing_area = gtk::DrawingArea::new();

        drawing_area.set_events(
            (EventMask::EXPOSURE_MASK
                | EventMask::POINTER_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::BUTTON_RELEASE_MASK
                | EventMask::KEY_PRESS_MASK
                | EventMask::ENTER_NOTIFY_MASK
                | EventMask::KEY_RELEASE_MASK
                | EventMask::SCROLL_MASK)
                .bits() as i32,
        );

        drawing_area.set_can_focus(true);
        drawing_area.grab_focus();

        drawing_area.connect_enter_notify_event(|widget, _| {
            widget.grab_focus();

            Inhibit(true)
        });

        {
            let mut last_size = Cell::new((0, 0));
            let handler = Arc::clone(&handler);
            drawing_area.connect_draw(move |widget, context| {
                let extents = context.clip_extents();
                let size = (
                    (extents.2 - extents.0) as u32,
                    (extents.3 - extents.1) as u32,
                );

                if last_size.get() != size {
                    last_size.set(size);
                    handler.size(size.0, size.1);
                }

                context.set_source_rgb(0.0, 0.0, 0.0);
                context.paint();

                context.set_source_rgb(1.0, 0.0, 0.0);
                context.rectangle(0.0, 0.0, 100.0, 100.0);

                context.fill();

                // For some reason piet needs a mutable context, so give it one I guess.
                let mut context = context.clone();
                let mut piet_context = Piet::new(&mut context);
                let anim = handler.paint(&mut piet_context);
                piet_context.finish();

                if anim {
                    widget.queue_draw();
                }

                Inhibit(false)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_button_press_event(move |_widget, button| {
                // XXX: what units is this in
                let position = button.get_position();
                handler.mouse(&window::MouseEvent {
                    x: position.0 as i32,
                    y: position.1 as i32,
                    mods: gtk_modifiers_to_druid(button.get_state()),
                    which: gtk_button_to_druid(button.get_button()),
                    ty: window::MouseType::Down,
                });

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_button_release_event(move |_widget, button| {
                // XXX: what units is this in
                let position = button.get_position();
                handler.mouse(&window::MouseEvent {
                    x: position.0 as i32,
                    y: position.1 as i32,
                    mods: gtk_modifiers_to_druid(button.get_state()),
                    which: gtk_button_to_druid(button.get_button()),
                    ty: window::MouseType::Up,
                });

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_motion_notify_event(move |_widget, motion| {
                let position = motion.get_position();

                handler.mouse_move(
                    position.0 as i32,
                    position.1 as i32,
                    gtk_modifiers_to_druid(motion.get_state()),
                );

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_scroll_event(move |_widget, scroll| {
                use gdk::ScrollDirection;
                let deltas = scroll.get_scroll_deltas();
                let modifiers = gtk_modifiers_to_druid(scroll.get_state());

                // The magic "120"s are from Microsoft's documentation for WM_MOUSEWHEEL.
                // They claim that one "tick" on a scroll wheel should be 120 units.
                // GTK simply reports the direction
                match scroll.get_direction() {
                    ScrollDirection::Up => {
                        handler.mouse_wheel(120, modifiers);
                    }
                    ScrollDirection::Down => {
                        handler.mouse_wheel(-120, modifiers);
                    }
                    ScrollDirection::Left => {
                        // Note: this direction is just a guess, I (bobtwinkles) don't
                        // have a way to test horizontal scroll events under GTK.
                        // If it's wrong, the right direction also needs to be changed
                        handler.mouse_hwheel(120, modifiers);
                    }
                    ScrollDirection::Right => {
                        handler.mouse_hwheel(-120, modifiers);
                    }
                    ScrollDirection::Smooth => {
                        eprintln!("Warning: somehow the Druid widget got a smooth scroll event");
                    }
                    e => {
                        eprintln!(
                            "Warning: the Druid widget got some whacky scroll direction {:?}",
                            e
                        );
                    }
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_key_press_event(move |_widget, key| {
                handler.keydown(
                    key.get_hardware_keycode() as i32,
                    gtk_modifiers_to_druid(key.get_state()),
                );

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_key_release_event(move |_widget, key| {
                let modifiers = gtk_modifiers_to_druid(key.get_state());

                match gdk::keyval_to_unicode(key.get_keyval()) {
                    Some(chr) => handler.char(chr as u32, modifiers),
                    None => {
                        use gdk::enums::key;

                        match key.get_keyval() {
                            key::KP_Enter => handler.char('\n' as u32, modifiers),
                            v => eprintln!("Warning: Druid got bogus key value {:?}", v),
                        }
                    }
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            drawing_area.connect_destroy(move |widget| {
                handler.destroy();
            });
        }

        window.add(&drawing_area);

        let tr = WindowHandle {
            window: Some(window),
        };

        handler.connect(&window::WindowHandle { inner: tr.clone() });

        Ok(tr)
    }
}

impl WindowHandle {
    pub fn show(&self) {
        use gtk::WidgetExt;
        match self.window.as_ref() {
            Some(window) => window.show_all(),
            None => return,
        }
        // self.window.map(|window| window.show_all());
    }

    /// Close the window.
    pub fn close(&self) {
        use gtk::GtkApplicationExt;
        match self.window.as_ref() {
            Some(window) => {
                with_application(|app| {
                    app.remove_window(window);
                });
            }
            None => return,
        }
    }

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        use gtk::WidgetExt;
        match self.window.as_ref() {
            Some(window) => window.queue_draw(),
            None => {}
        }
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        unimplemented!("WindowHandle::get_idle_handle");
    }

    /// Get the dpi of the window.
    ///
    /// TODO: we want to migrate this from dpi (with 96 as nominal) to a scale
    /// factor (with 1 as nominal).
    pub fn get_dpi(&self) -> f32 {
        // TODO: get actual dpi
        96.0
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

    pub fn file_dialog(
        &self,
        ty: FileDialogType,
        options: FileDialogOptions,
    ) -> Result<OsString, Error> {
        unimplemented!()
    }
}

unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the main UI thread, and
    /// won't be scheduled if the associated view has been dropped.
    ///
    /// Note: the name "idle" suggests that it will be scheduled with a lower
    /// priority than other UI events, but that's not necessarily the case.
    pub fn add_idle<F>(&self, callback: F)
    where
        F: FnOnce(&Any) + Send + 'static,
    {
        if let Some(queue) = self.idle_queue.upgrade() {
            let mut queue = queue.lock().unwrap();
            if queue.is_empty() {
                unimplemented!("Idle queue wait");
            }
            queue.push(Box::new(callback));
        }
    }
}

/// Map a GTK mouse button to a Druid one
#[inline]
fn gtk_button_to_druid(button: u32) -> window::MouseButton {
    use window::MouseButton;
    match button {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        4 => MouseButton::X1,
        _ => MouseButton::X2,
    }
}

/// Map the GTK modifiers into Druid bits
#[inline]
fn gtk_modifiers_to_druid(modifiers: gdk::ModifierType) -> u32 {
    use gdk::ModifierType;
    use keycodes;

    let mut output = 0;

    if modifiers.contains(ModifierType::MOD1_MASK) {
        output |= keycodes::M_ALT;
    }
    if modifiers.contains(ModifierType::CONTROL_MASK) {
        output |= keycodes::M_CTRL;
    }
    if modifiers.contains(ModifierType::SHIFT_MASK) {
        output |= keycodes::M_SHIFT;
    }

    output
}
