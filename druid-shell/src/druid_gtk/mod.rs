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
use std::ffi::OsString;
use std::cell::Cell;
use std::sync::{Arc, Mutex, Weak};

use gtk::ApplicationWindow;

use piet::RenderContext;
use piet_common::Piet;

use platform::dialog::{FileDialogOptions, FileDialogType};
use platform::menu::Menu;
use window::{Cursor, WinHandler};
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

        {
            let mut last_size = Cell::new((0, 0));
            let handler = Arc::clone(&handler);
            drawing_area.connect_draw(move |widget, context| {
                use gtk::Inhibit;

                let extents = context.clip_extents();
                let size = ((extents.2 - extents.0) as u32, (extents.3 - extents.1) as u32);

                if last_size.get() != size {
                    last_size.set(size);
                    handler.size(size.0, size.1);
                }

                eprintln!("Clip extents: {:?}", context.clip_extents());

                context.set_source_rgb(0.0, 0.0, 0.0);
                context.paint();

                context.set_source_rgb(1.0, 0.0, 0.0);
                context.rectangle(0.0, 0.0, 100.0, 100.0);

                context.fill();

                // For some reason piet needs a mutable context, so give it one I guess.
                let mut context = context.clone();
                let mut piet_context = Piet::new(&mut context);
                handler.paint(&mut piet_context);

                Inhibit(false)
            });
        }

        window.add(&drawing_area);

        Ok(WindowHandle {
            window: Some(window),
        })
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
        unimplemented!("WindowHandle::invalidate");
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
