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

//! Web window creation and management.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use instant::Instant;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::kurbo::{Point, Rect, Size, Vec2};

use crate::piet::{RenderContext, PietText};

use super::application::Application;
use super::error::Error;
use super::keycodes::convert_keyboard_event;
use super::menu::Menu;
use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::error::Error as ShellError;
use crate::scale::{Scale, ScaledArea};

use crate::keyboard::{KbKey, KeyState, Modifiers};
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
use crate::region::Region;
use crate::window::{IdleToken, TimerToken, WinHandler};
use crate::window;

// This is a macro instead of a function since KeyboardEvent and MouseEvent has identical functions
// to query modifier key states.
macro_rules! get_modifiers {
    ($event:ident) => {{
        let mut result = Modifiers::default();
        result.set(Modifiers::SHIFT, $event.shift_key());
        result.set(Modifiers::ALT, $event.alt_key());
        result.set(Modifiers::CONTROL, $event.ctrl_key());
        result.set(Modifiers::META, $event.meta_key());
        result.set(Modifiers::ALT_GRAPH, $event.get_modifier_state("AltGraph"));
        result.set(Modifiers::CAPS_LOCK, $event.get_modifier_state("CapsLock"));
        result.set(Modifiers::NUM_LOCK, $event.get_modifier_state("NumLock"));
        result.set(
            Modifiers::SCROLL_LOCK,
            $event.get_modifier_state("ScrollLock"),
        );
        result
    }};
}

/// Builder abstraction for creating new windows.
pub(crate) struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    cursor: Cursor,
    menu: Option<Menu>,
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<WindowState>);

/// A handle that can get used to schedule an idle handler. Note that
/// this handle is thread safe.
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
    scale: Cell<Scale>,
    area: Cell<ScaledArea>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    handler: RefCell<Box<dyn WinHandler>>,
    window: web_sys::Window,
    canvas: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
    invalid: RefCell<Region>,
}

impl WindowState {
    fn render(&self) {
        self.handler.borrow_mut().prepare_paint();

        let mut piet_ctx = piet_common::Piet::new(self.context.clone(), self.window.clone());
        if let Err(e) = piet_ctx.with_save(|mut ctx| {
            let invalid = self.invalid.borrow();
            ctx.clip(invalid.to_bez_path());
            self.handler.borrow_mut().paint(&mut ctx, &invalid);
            Ok(())
        }) {
            log::error!("piet error on render: {:?}", e);
        }
        if let Err(e) = piet_ctx.finish() {
            log::error!("piet error finishing render: {:?}", e);
        }
        self.invalid.borrow_mut().clear();
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

    fn request_animation_frame(&self, f: impl FnOnce() + 'static) -> Result<i32, Error> {
        Ok(self
            .window
            .request_animation_frame(Closure::once_into_js(f).as_ref().unchecked_ref())?)
    }

    /// Returns the window size in css units
    fn get_window_size_and_dpr(&self) -> (f64, f64, f64) {
        let w = &self.window;
        let width = w.inner_width().unwrap().as_f64().unwrap();
        let height = w.inner_height().unwrap().as_f64().unwrap();
        let dpr = w.device_pixel_ratio();
        (width, height, dpr)
    }

    /// Updates the canvas size and scale factor and returns `Scale` and `ScaledArea`.
    fn update_scale_and_area(&self) -> (Scale, ScaledArea) {
        let (css_width, css_height, dpr) = self.get_window_size_and_dpr();
        let scale = Scale::new(dpr, dpr);
        let area = ScaledArea::from_dp(Size::new(css_width, css_height), scale);
        let size_px = area.size_px();
        self.canvas.set_width(size_px.width as u32);
        self.canvas.set_height(size_px.height as u32);
        let _ = self.context.scale(scale.x(), scale.y());
        self.scale.set(scale);
        self.area.set(area);
        (scale, area)
    }
}

fn setup_mouse_down_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_canvas_event_listener(ws, "mousedown", move |event: web_sys::MouseEvent| {
        if let Some(button) = mouse_button(event.button()) {
            let buttons = mouse_buttons(event.buttons());
            let event = MouseEvent {
                pos: Point::new(event.offset_x() as f64, event.offset_y() as f64),
                buttons,
                mods: get_modifiers!(event),
                count: 1,
                focus: false,
                button,
                wheel_delta: Vec2::ZERO,
            };
            state.handler.borrow_mut().mouse_down(&event);
        }
    });
}

fn setup_mouse_up_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_canvas_event_listener(ws, "mouseup", move |event: web_sys::MouseEvent| {
        if let Some(button) = mouse_button(event.button()) {
            let buttons = mouse_buttons(event.buttons());
            let event = MouseEvent {
                pos: Point::new(event.offset_x() as f64, event.offset_y() as f64),
                buttons,
                mods: get_modifiers!(event),
                count: 0,
                focus: false,
                button,
                wheel_delta: Vec2::ZERO,
            };
            state.handler.borrow_mut().mouse_up(&event);
        }
    });
}

fn setup_mouse_move_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_canvas_event_listener(ws, "mousemove", move |event: web_sys::MouseEvent| {
        let buttons = mouse_buttons(event.buttons());
        let event = MouseEvent {
            pos: Point::new(event.offset_x() as f64, event.offset_y() as f64),
            buttons,
            mods: get_modifiers!(event),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        };
        state.handler.borrow_mut().mouse_move(&event);
    });
}

fn setup_scroll_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_canvas_event_listener(ws, "wheel", move |event: web_sys::WheelEvent| {
        let delta_mode = event.delta_mode();

        let dx = event.delta_x();
        let dy = event.delta_y();

        // The value 35.0 was manually picked to produce similar behavior to mac/linux.
        let wheel_delta = match delta_mode {
            web_sys::WheelEvent::DOM_DELTA_PIXEL => Vec2::new(dx, dy),
            web_sys::WheelEvent::DOM_DELTA_LINE => Vec2::new(35.0 * dx, 35.0 * dy),
            web_sys::WheelEvent::DOM_DELTA_PAGE => {
                let size_dp = state.area.get().size_dp();
                Vec2::new(size_dp.width * dx, size_dp.height * dy)
            }
            _ => {
                log::warn!("Invalid deltaMode in WheelEvent: {}", delta_mode);
                return;
            }
        };

        let event = MouseEvent {
            pos: Point::new(event.offset_x() as f64, event.offset_y() as f64),
            buttons: mouse_buttons(event.buttons()),
            mods: get_modifiers!(event),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta,
        };
        state.handler.borrow_mut().wheel(&event);
    });
}

fn setup_resize_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_window_event_listener(ws, "resize", move |_: web_sys::UiEvent| {
        let (scale, area) = state.update_scale_and_area();
        // TODO: For performance, only call the handler when these values actually changed.
        state.handler.borrow_mut().scale(scale);
        state.handler.borrow_mut().size(area.size_dp());
    });
}

fn setup_keyup_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_window_event_listener(ws, "keyup", move |event: web_sys::KeyboardEvent| {
        let modifiers = get_modifiers!(event);
        let kb_event = convert_keyboard_event(&event, modifiers, KeyState::Up);
        state.handler.borrow_mut().key_up(kb_event);
    });
}

fn setup_keydown_callback(ws: &Rc<WindowState>) {
    let state = ws.clone();
    register_window_event_listener(ws, "keydown", move |event: web_sys::KeyboardEvent| {
        let modifiers = get_modifiers!(event);
        let kb_event = convert_keyboard_event(&event, modifiers, KeyState::Down);
        if kb_event.key == KbKey::Backspace {
            // Prevent the browser from going back a page by default.
            event.prevent_default();
        }
        state.handler.borrow_mut().key_down(kb_event);
    });
}

/// A helper function to register a window event listener with `addEventListener`.
fn register_window_event_listener<F, E>(window_state: &Rc<WindowState>, event_type: &str, f: F)
where
    F: 'static + FnMut(E),
    E: 'static + wasm_bindgen::convert::FromWasmAbi,
{
    let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(_)>);
    window_state
        .window
        .add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// A helper function to register a canvas event listener with `addEventListener`.
fn register_canvas_event_listener<F, E>(window_state: &Rc<WindowState>, event_type: &str, f: F)
where
    F: 'static + FnMut(E),
    E: 'static + wasm_bindgen::convert::FromWasmAbi,
{
    let closure = Closure::wrap(Box::new(f) as Box<dyn FnMut(_)>);
    window_state
        .canvas
        .add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn setup_web_callbacks(window_state: &Rc<WindowState>) {
    setup_mouse_down_callback(window_state);
    setup_mouse_move_callback(window_state);
    setup_mouse_up_callback(window_state);
    setup_resize_callback(window_state);
    setup_scroll_callback(window_state);
    setup_keyup_callback(window_state);
    setup_keydown_callback(window_state);
}

impl WindowBuilder {
    pub fn new(_app: Application) -> WindowBuilder {
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

    pub fn set_position(&mut self, _position: Point) {
        // Ignored
    }

    pub fn set_window_state(&self, _state: window::WindowState) {
        // Ignored
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
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
        // Create the Scale for resolution scaling
        let scale = {
            let dpr = window.device_pixel_ratio();
            Scale::new(dpr, dpr)
        };
        let area = {
            // The initial size in display points isn't necessarily the final size in display points
            let size_dp = Size::new(canvas.offset_width() as f64, canvas.offset_height() as f64);
            ScaledArea::from_dp(size_dp, scale)
        };
        let size_px = area.size_px();
        canvas.set_width(size_px.width as u32);
        canvas.set_height(size_px.height as u32);
        let _ = context.scale(scale.x(), scale.y());
        let size_dp = area.size_dp();

        set_cursor(&canvas, &self.cursor);

        let handler = self.handler.unwrap();

        let window = Rc::new(WindowState {
            scale: Cell::new(scale),
            area: Cell::new(area),
            idle_queue: Default::default(),
            handler: RefCell::new(handler),
            window,
            canvas,
            context,
            invalid: RefCell::new(Region::EMPTY),
        });

        setup_web_callbacks(&window);

        // Register the scale & size with the window handler.
        let wh = window.clone();
        window
            .request_animation_frame(move || {
                wh.handler.borrow_mut().scale(scale);
                wh.handler.borrow_mut().size(size_dp);
            })
            .expect("Failed to request animation frame");

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
        log::warn!("resizable unimplemented for web");
    }

    pub fn show_titlebar(&self, _show_titlebar: bool) {
        log::warn!("show_titlebar unimplemented for web");
    }

    pub fn set_position(&self, _position: Point) {
        log::warn!("WindowHandle::set_position unimplemented for web");
    }

    pub fn get_position(&self) -> Point {
        log::warn!("WindowHandle::get_position unimplemented for web.");
        Point::new(0.0, 0.0)
    }

    pub fn set_size(&self, _size: Size) {
        log::warn!("WindowHandle::set_size unimplemented for web.");
    }

    pub fn get_size(&self) -> Size {
        log::warn!("WindowHandle::get_size unimplemented for web.");
        Size::new(0.0, 0.0)
    }

    pub fn set_window_state(&self, _state: window::WindowState) {
        log::warn!("WindowHandle::set_window_state unimplemented for web.");
    }

    pub fn get_window_state(&self) -> window::WindowState {
        log::warn!("WindowHandle::get_window_state unimplemented for web.");
        window::WindowState::RESTORED
    }

    pub fn handle_titlebar(&self, _val: bool) {
        log::warn!("WindowHandle::handle_titlebar unimplemented for web.");
    }

    pub fn close(&self) {
        // TODO
    }

    pub fn bring_to_front_and_focus(&self) {
        log::warn!("bring_to_frontand_focus unimplemented for web");
    }

    pub fn request_anim_frame(&self) {
        self.render_soon();
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(s) = self.0.upgrade() {
            s.invalid.borrow_mut().add_rect(rect);
        }
        self.render_soon();
    }

    pub fn invalidate(&self) {
        if let Some(s) = self.0.upgrade() {
            s.invalid
                .borrow_mut()
                .add_rect(s.area.get().size_dp().to_rect());
        }
        self.render_soon();
    }

    pub fn text(&self) -> PietText {
        let s = self
            .0
            .upgrade()
            .unwrap_or_else(|| panic!("Failed to produce a text context"));

        PietText::new(s.context.clone())
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        use std::convert::TryFrom;
        let interval = deadline.duration_since(Instant::now()).as_millis();
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
            state
                .window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    Closure::once_into_js(f).as_ref().unchecked_ref(),
                    interval,
                )
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
        log::warn!("open_file_sync is currently unimplemented for web.");
        self.file_dialog(FileDialogType::Open, options)
            .ok()
            .map(|s| FileInfo { path: s.into() })
    }

    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        log::warn!("save_as_sync is currently unimplemented for web.");
        self.file_dialog(FileDialogType::Save, options)
            .ok()
            .map(|s| FileInfo { path: s.into() })
    }

    fn render_soon(&self) {
        if let Some(s) = self.0.upgrade() {
            let state = s.clone();
            s.request_animation_frame(move || {
                state.render();
            })
            .expect("Failed to request animation frame");
        }
    }

    pub fn file_dialog(
        &self,
        _ty: FileDialogType,
        _options: FileDialogOptions,
    ) -> Result<OsString, ShellError> {
        Err(ShellError::Platform(Error::Unimplemented))
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.0.upgrade().map(|w| IdleHandle {
            state: Rc::downgrade(&w),
            queue: w.idle_queue.clone(),
        })
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        Ok(self
            .0
            .upgrade()
            .ok_or(ShellError::WindowDropped)?
            .scale
            .get())
    }

    pub fn set_menu(&self, _menu: Menu) {
        log::warn!("set_menu unimplemented for web");
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        log::warn!("show_context_menu unimplemented for web");
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
                window_state
                    .request_animation_frame(move || {
                        state.process_idle_queue();
                    })
                    .expect("request_animation_frame failed");
            }
        }
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        let mut queue = self.queue.lock().expect("IdleHandle::add_idle queue");
        queue.push(IdleKind::Token(token));

        if queue.len() == 1 {
            if let Some(window_state) = self.state.upgrade() {
                let state = window_state.clone();
                window_state
                    .request_animation_frame(move || {
                        state.process_idle_queue();
                    })
                    .expect("request_animation_frame failed");
            }
        }
    }
}

fn mouse_button(button: i16) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        3 => Some(MouseButton::X1),
        4 => Some(MouseButton::X2),
        _ => None,
    }
}

fn mouse_buttons(mask: u16) -> MouseButtons {
    let mut buttons = MouseButtons::new();
    if mask & 1 != 0 {
        buttons.insert(MouseButton::Left);
    }
    if mask & 1 << 1 != 0 {
        buttons.insert(MouseButton::Right);
    }
    if mask & 1 << 2 != 0 {
        buttons.insert(MouseButton::Middle);
    }
    if mask & 1 << 3 != 0 {
        buttons.insert(MouseButton::X1);
    }
    if mask & 1 << 4 != 0 {
        buttons.insert(MouseButton::X2);
    }
    buttons
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
            },
        )
        .unwrap_or_else(|_| log::warn!("Failed to set cursor"));
}
