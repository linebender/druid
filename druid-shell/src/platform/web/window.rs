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

//! Web window creation and management.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use instant::Instant;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::kurbo::{Size, Point};

use crate::piet::RenderContext;

use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use super::menu::Menu;
use super::error::Error;
use crate::common_util::IdleCallback;

use crate::KeyModifiers;
use crate::window::{WinHandler, IdleToken, Text, TimerToken};
use crate::mouse::{Cursor, MouseButton, MouseEvent};

use log::{error, warn};
use super::util::init_log;

type Result<T> = std::result::Result<T, Error>;

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    cursor: Cursor,
    menu: Option<Menu>,
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<WindowState>);

/// A handle that can get used to schedule an idle handler. Note that
/// this handle is thread safe. If the handle is used after the hwnd
/// has been destroyed, probably not much will go wrong (the XI_RUN_IDLE
/// message may be sent to a stray window).
#[derive(Clone)]
pub struct IdleHandle {
    state: Weak<WindowState>,
    queue: Arc<Mutex<Vec<IdleKind>>>,
}

enum IdleKind {
    Callback(Box<dyn IdleCallback>),
    Token(IdleToken),
}

struct WindowState {
    dpi: Cell<f32>,
    dpr: Cell<f64>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    handler: RefCell<Box<dyn WinHandler>>,
    window: web_sys::Window,
    canvas: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
}

impl WindowState {
    fn render(&self) -> bool {
        self.context.clear_rect(0.0, 0.0, self.get_width() as f64, self.get_height() as f64);
        let mut piet_ctx = piet_common::Piet::new(self.context.clone(), self.window.clone());
        let want_anim_frame = self.handler.borrow_mut().paint(&mut piet_ctx);
        if let Err(e) = piet_ctx.finish() {
            error!("piet error on render: {:?}", e);
        }
        let res = piet_ctx.finish();
        if let Err(e) = res {
            error!("EndDraw error: {:?}", e);
        }
        want_anim_frame
    }

    fn process_idle_queue(&self) {
        let mut queue = self.idle_queue.lock().expect("process_idle_queue");
        for item in queue.drain(..) {
            match item {
                IdleKind::Callback(cb) => cb.call(&self.handler),
                IdleKind::Token(tok) => self.handler.borrow_mut().idle(tok),
            }
        }
    }

    fn get_width(&self) -> u32 {
        (self.canvas.offset_width() as f64 * self.dpr.get()) as u32
    }

    fn get_height(&self) -> u32 {
        (self.canvas.offset_height() as f64 * self.dpr.get()) as u32
    }

    fn request_animation_frame(&self, f: impl FnOnce() + 'static) -> Result<i32> {
        Ok(self.window.request_animation_frame(Closure::once_into_js(f).as_ref().unchecked_ref())?)
    }

    /// Returns the window size in css units
    fn get_window_size_and_dpr(&self) -> (f64, f64, f64) {
        let w = &self.window;
        let width = w.inner_width().unwrap().as_f64().unwrap();
        let height = w.inner_height().unwrap().as_f64().unwrap();
        let dpr = w.device_pixel_ratio();
        (width, height, dpr)
    }

}

fn setup_mouse_down_callback(window_state: &Rc<WindowState>) {
    let state = window_state.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let button = mouse_button(event.button()).unwrap();
        let dpr = state.dpr.get();
        let event = MouseEvent {
            pos: Point::new(
                dpr * event.offset_x() as f64,
                dpr * event.offset_y() as f64),
            mods: KeyModifiers::default(),
            button,
            count: 1,
        };
        state.handler.borrow_mut().mouse_down(&event);
    }) as Box<dyn FnMut(_)>);
    window_state.canvas
        .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn setup_mouse_move_callback(window_state: &Rc<WindowState>) {
    let state = window_state.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let button = mouse_button(event.button()).unwrap();
        let dpr = state.dpr.get();
        let event = MouseEvent {
            pos: Point::new(
                dpr * event.offset_x() as f64,
                dpr * event.offset_y() as f64),
            mods: KeyModifiers::default(),
            button,
            count: 1,
        };
	state.handler.borrow_mut().mouse_move(&event);
    }) as Box<dyn FnMut(_)>);
    window_state.canvas
        .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn setup_mouse_up_callback(window_state: &Rc<WindowState>) {
    let state = window_state.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let button = mouse_button(event.button()).unwrap();
        let dpr = state.dpr.get();
        let event = MouseEvent {
            pos: Point::new(
                     dpr * event.offset_x() as f64,
                     dpr * event.offset_y() as f64),
            mods: KeyModifiers::default(),
            button,
            count: 0,
        };
        state.handler.borrow_mut().mouse_up(&event);
    }) as Box<dyn FnMut(_)>);
    window_state.canvas
        .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
}

fn setup_resize_callback(window_state: &Rc<WindowState>) {
    let state = window_state.clone();
    let closure = Closure::wrap(Box::new(move |_: web_sys::UiEvent| {
        let (css_width, css_height, dpr) = state.get_window_size_and_dpr();
        let physical_width = (dpr * css_width) as u32;
        let physical_height = (dpr * css_height) as u32;
        state.dpr.replace(dpr);
        state.canvas.set_width(physical_width);
        state.canvas.set_height(physical_height);
        state.handler.borrow_mut().size(physical_width, physical_height);
    }) as Box<dyn FnMut(_)>);
    window_state.canvas
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn setup_web_callbacks(window_state: &Rc<WindowState>) {
    setup_mouse_down_callback(window_state);
    setup_mouse_move_callback(window_state);
    setup_mouse_up_callback(window_state);
    setup_resize_callback(window_state);
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            cursor: Cursor::Arrow,
            menu: None,
        }
    }

    /// This takes ownership, and is typically used with UiMain
    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, _: Size) {
        // Ignored
    }

    pub fn set_min_size(&mut self, _: Size) {
        // Ignored
    }

    pub fn resizable(&mut self, _resizable: bool) {
        // Ignored
    }

    pub fn show_titlebar(&mut self, _show_titlebar: bool) {
        // Ignored
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle> {
        // Initialize logging to the browser console.
        init_log();

        let window = web_sys::window().ok_or_else(|| Error::NoWindow)?;
        let canvas = window
            .document()
            .ok_or(Error::NoDocument)?
            .get_element_by_id("canvas")
            .ok_or_else(|| Error::NoElementById("canvas".to_string()))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| Error::JsCast)?;
        let context = canvas
            .get_context("2d")?
            .ok_or(Error::NoContext)?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .map_err(|_| Error::JsCast)?;

        let dpr = window.device_pixel_ratio();
        let old_w = canvas.offset_width();
        let old_h = canvas.offset_height();
        let new_w = (old_w as f64 * dpr) as u32;
        let new_h = (old_h as f64 * dpr) as u32;

        canvas.set_width(new_w);
        canvas.set_height(new_h);
        set_cursor(&canvas, &self.cursor);
        //let _ = context.scale(dpr, dpr);

        let handler = self.handler.unwrap();

        let window = Rc::new(WindowState {
            dpi: Cell::new(96.0),
            dpr: Cell::new(dpr),
            idle_queue: Default::default(),
            handler: RefCell::new(handler),
            window,
            canvas,
            context,
        });

        setup_web_callbacks(&window);

        // Register the size with the window handler.
        let wh = window.clone();
        window.request_animation_frame(move || {
            wh.handler.borrow_mut().size(new_w, new_h);
        }).expect("Failed to request animation frame");

        let handle = WindowHandle(Rc::downgrade(&window));

        window.handler.borrow_mut().connect(&handle.clone().into());

        Ok(handle)
    }
}

impl WindowHandle {
    pub fn show(&self) {
        self.render_soon();
    }

    pub fn resizable(&self, _resizable: bool) {
        warn!("resizable unimplemented for web");
    }

    pub fn show_titlebar(&self, _show_titlebar: bool) {
        warn!("show_titlebar unimplemented for web");
    }

    pub fn close(&self) {
        // TODO
    }

    pub fn bring_to_front_and_focus(&self) {
        warn!("bring_to_frontand_focus unimplemented for web");
    }

    pub fn invalidate(&self) {
        self.render_soon();
    }

    pub fn text(&self) -> Text {
        let s = self.0.upgrade().unwrap_or_else(|| {
            panic!("Failed to produce a text context")
        });

        Text::new(s.context.clone(), s.window.clone())
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        use std::convert::TryFrom;
        let interval = deadline
            .duration_since(Instant::now())
            .as_millis();
        let interval = match i32::try_from(interval) {
            Ok(iv) => iv,
            Err(_) => {
                log::warn!("Timer duration exceeds 32 bit integer max");
                i32::max_value()
            }
        };

        let token = TimerToken::next();

        if let Some(state) = self.0.upgrade() {
            let s = state.clone();
            let f = move || {
                if let Ok(mut handler_borrow) = s.handler.try_borrow_mut() {
                    handler_borrow.timer(token);
                }
            };
            state.window.set_timeout_with_callback_and_timeout_and_arguments_0(
                Closure::once_into_js(f).as_ref().unchecked_ref(), interval)
                .expect("Failed to call setTimeout with a callback");
        }
        token
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(s) = self.0.upgrade() {
            set_cursor(&s.canvas, cursor);
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


    fn render_soon(&self) {
        if let Some(s) = self.0.upgrade() {
            let handle = self.clone();
            let state = s.clone();
            s.request_animation_frame(move || {
                let want_anim_frame = state.render();
                log::debug!("want_anim_frame: {}", want_anim_frame);
                if want_anim_frame {
                    handle.render_soon();
                }
            }).expect("Failed to request animation frame");
        }
    }

    pub fn file_dialog(
        &self,
        _ty: FileDialogType,
        _options: FileDialogOptions,
    ) -> std::result::Result<OsString, crate::Error> {
        Err(crate::Error::Platform(Error::Unimplemented))
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.0.upgrade().map(|w| IdleHandle {
            state: Rc::downgrade(&w),
            queue: w.idle_queue.clone(),
        })
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


    pub fn set_menu(&self, _menu: Menu) {
        warn!("set_menu unimplemented for web");
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        warn!("show_context_menu unimplemented for web");
    }

    pub fn set_title(&self, title: impl Into<String>) {
        if let Some(state) = self.0.upgrade() {
            state.canvas.set_title(&(title.into()))
        }
    }
}

unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the main thread is idle.
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        let mut queue = self.queue.lock().expect("IdleHandle::add_idle queue");
        queue.push(IdleKind::Callback(Box::new(callback)));

        if queue.len() == 1 {
            if let Some(window_state) = self.state.upgrade() {
                let state = window_state.clone();
                window_state.request_animation_frame(move || {
                    state.process_idle_queue();
                }).expect("request_animation_frame failed");
            }
        }
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        let mut queue = self.queue.lock().expect("IdleHandle::add_idle queue");
        queue.push(IdleKind::Token(token));

        if queue.len() == 1 {
            if let Some(window_state) = self.state.upgrade() {
                let state = window_state.clone();
                window_state.request_animation_frame(move || {
                    state.process_idle_queue();
                }).expect("request_animation_frame failed");
            }
        }
    }
}

fn mouse_button(button: i16) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        _ => None
    }
}

fn set_cursor(canvas: &web_sys::HtmlCanvasElement, cursor: &Cursor) {
    canvas
        .style()
        .set_property(
        "cursor",
        match cursor {
            Cursor::Arrow => "default",
            Cursor::IBeam => "text",
            Cursor::Crosshair => "crosshair",
            Cursor::OpenHand => "grab",
            Cursor::NotAllowed => "not-allowed",
            Cursor::ResizeLeftRight => "ew-resize",
            Cursor::ResizeUpDown => "ns-resize",
        }
    ).unwrap_or_else(|_| warn!("Failed to set cursor"));
}
