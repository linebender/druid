// Copyright 2018 The xi-editor Authors.
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

//! Creation and management of windows.

#![allow(non_snake_case)]

pub mod application;
pub mod dialog;
pub mod menu;
pub mod util;
pub mod win_main;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement};

use piet::RenderContext;

use dialog::{FileDialogOptions, FileDialogType};
use crate::menu::Menu;
use crate::Error;

use crate::window::{self, Cursor, MouseButton, MouseEvent, MouseType, WinHandler};

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<WinHandler>>,
    title: String,
    cursor: Cursor,
    menu: Option<Menu>,
    present_strategy: PresentStrategy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// It's very tricky to get smooth dynamics (especially resizing) and
/// good performance on Windows. This setting lets clients experiment
/// with different strategies.
pub enum PresentStrategy {
    /// Don't try to use DXGI at all, only create Hwnd render targets.
    /// Note: on Windows 7 this is the only mode available.
    Hwnd,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_SEQUENTIAL. In
    /// testing, it causes diagonal banding artifacts with Nvidia
    /// adapters, and incremental present doesn't work. However, it
    /// is compatible with GDI (such as menus).
    Sequential,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL.
    /// In testing, it seems to perform well (including allowing smooth
    /// resizing when the frame can be rendered quickly), but isn't
    /// compatible with GDI.
    Flip,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL
    /// but with a redirection surface for GDI compatibility. Resize is
    /// very laggy and artifacty.
    FlipRedirect,
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<WindowState>);

/// A handle that can get used to schedule an idle handler. Note that
/// this handle is thread safe. If the handle is used after the hwnd
/// has been destroyed, probably not much will go wrong (the XI_RUN_IDLE
/// message may be sent to a stray window).
#[derive(Clone)]
pub struct IdleHandle {
    queue: Arc<Mutex<Vec<Box<IdleCallback>>>>,
}

trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &Any);
}

impl<F: FnOnce(&Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &Any) {
        (*self)(a)
    }
}

struct WindowState {
    dpi: Cell<f32>,
    dpr: Cell<f64>,
    idle_queue: Arc<Mutex<Vec<Box<IdleCallback>>>>,
    handler: Box<WinHandler>,

    canvas: web_sys::HtmlCanvasElement,
    canvas_ctx: RefCell<web_sys::CanvasRenderingContext2d>,
}

impl Default for PresentStrategy {
    fn default() -> PresentStrategy {
        // We probably want to change this, but we need GDI to work. Too bad about
        // the artifacty resizing.
        PresentStrategy::FlipRedirect
    }
}

impl WindowState {
    // Renders but does not present.
    fn render(&self) {
        let window = web_sys::window().expect("web_sys window");
        let ref mut canvas_ctx = *self.canvas_ctx.borrow_mut();
        canvas_ctx.clear_rect(0.0, 0.0, self.get_width() as f64, self.get_height() as f64);
        let mut piet_ctx = piet_common::Piet::new(
            canvas_ctx, &window);
        self.handler.paint(&mut piet_ctx);
        if let Err(e) = piet_ctx.finish() {
            // TODO: use proper log infrastructure
            eprintln!("piet error on render: {:?}", e);
        }
        let res = piet_ctx.finish();
        if let Err(e) = res {
            println!("EndDraw error: {:?}", e);
        }
    }

    fn process_idle_queue(&self) {
        let mut queue = self.idle_queue.lock().expect("process_idle_queue");
        for callback in queue.drain(..) {
            web_sys::console::log_1(&"idle callback".into());
            callback.call(&self.handler);
        }
    }

    fn get_width(&self) -> u32 {
        (self.canvas.offset_width() as f64 * self.dpr.get()) as u32
    }

    fn get_height(&self) -> u32 {
        (self.canvas.offset_height() as f64 * self.dpr.get()) as u32
    }
}

fn setup_web_callbacks(window_state: &Rc<WindowState>) {
    {
        let state = window_state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let button = match event.button() {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                _ => { return; },
            };

            let event = MouseEvent {
                x: event.offset_x() as i32,
                y: event.offset_y() as i32,
                mods: 0,
                which: button,
                ty: MouseType::Down,
            };
            state.handler.mouse(&event);

            state.process_idle_queue();
            state.render();
        }) as Box<dyn FnMut(_)>);
        window_state.canvas
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }
    {
        let state = window_state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let x = event.offset_x() as i32;
	    let y = event.offset_y() as i32;
	    let mods = 0;
	    state.handler.mouse_move(x, y, mods);

            state.process_idle_queue();
            state.render();
        }) as Box<dyn FnMut(_)>);
        window_state.canvas
            .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }
    {
        let state = window_state.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let button = match event.button() {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                _ => { return; },
            };

            let event = MouseEvent {
                x: event.offset_x() as i32,
                y: event.offset_y() as i32,
                mods: 0,
                which: button,
                ty: MouseType::Up,
            };
            state.handler.mouse(&event);

            state.process_idle_queue();
            state.render();
        }) as Box<dyn FnMut(_)>);
        window_state.canvas
            .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            cursor: Cursor::Arrow,
            menu: None,
            present_strategy: Default::default(),
        }
    }

    /// This takes ownership, and is typically used with UiMain
    pub fn set_handler(&mut self, handler: Box<WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_scroll(&mut self, hscroll: bool, vscroll: bool) {
        // TODO
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    /// Set the default cursor for the window.
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn set_present_strategy(&mut self, present_strategy: PresentStrategy) {
        self.present_strategy = present_strategy;
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        let window = window().expect("web_sys window");
        let canvas = window
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
        let mut context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let dpr = window.device_pixel_ratio();
        let canvas_w = (canvas.offset_width() as f64 * dpr) as u32;
        let canvas_h = (canvas.offset_height() as f64 * dpr) as u32;
        canvas.set_width(canvas_w);
        canvas.set_height(canvas_h);
        let _ = context.scale(dpr, dpr);

        let handler = self.handler.unwrap();

        handler.size(canvas_w, canvas_h);

        let window = Rc::new(WindowState {
            dpi: Cell::new(96.0),
            dpr: Cell::new(dpr),
            idle_queue: Default::default(),
            handler: handler,
            canvas: canvas,
            canvas_ctx: RefCell::new(context),
        });

        setup_web_callbacks(&window);

        Ok(WindowHandle(Rc::downgrade(&window)))
    }
}

impl WindowHandle {
    pub fn show(&self) {
        if let Some(w) = self.0.upgrade() {
            w.render();
        }
    }

    pub fn close(&self) {
        // TODO
    }

    pub fn invalidate(&self) {
        if let Some(w) = self.0.upgrade() {
            w.render();
        }
    }

    pub fn file_dialog(
        &self,
        ty: FileDialogType,
        options: FileDialogOptions,
    ) -> Result<OsString, Error> {
        Err(Error::Null)
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.0.upgrade().map(|w| IdleHandle {
            queue: w.idle_queue.clone(),
        })
    }

    fn take_idle_queue(&self) -> Vec<Box<IdleCallback>> {
        if let Some(w) = self.0.upgrade() {
            mem::replace(&mut w.idle_queue.lock().unwrap(), Vec::new())
        } else {
            Vec::new()
        }
    }

    /// Get the dpi of the window.
    pub fn get_dpi(&self) -> f32 {
        if let Some(w) = self.0.upgrade() {
            w.dpi.get()
        } else {
            96.0
        }
    }

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
}

unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty.
    pub fn add_idle<F>(&self, callback: F)
    where
        F: FnOnce(&Any) + Send + 'static,
    {
        let mut queue = self.queue.lock().expect("add_idle queue");
        queue.push(Box::new(callback));
    }
}
