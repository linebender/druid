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

//! Window creation and management.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::panic::Location;

use instant::Instant;

use crate::kurbo::{Point, Rect, Size};

use crate::piet::{Piet, PietText, RenderContext};

use super::application::Application;
use super::error::Error;
use super::menu::Menu;
use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType};
use crate::error::Error as ShellError;
use crate::scale::{Scale, ScaledArea};

use crate::mouse::{Cursor, CursorDesc, MouseButton, MouseButtons};
use crate::region::Region;
use crate::window;
use crate::window::{FileDialogToken, IdleToken, TimerToken, WinHandler, WindowLevel};

use skulpin::skia_safe;
use skulpin::winit;

pub struct Window {
    handler: RefCell<Box<dyn WinHandler>>,
    window_state: RefCell<WindowState>,
}

impl Window {
    //// TODO passing winit window in this struct is not ok....
    //pub fn render(&mut self, window: &skulpin::WinitWindow, control_flow: &mut winit::event_loop::ControlFlow) {
    //        dbg!("I'm rendering stuff");
    //        let mut piet_ctx = Piet::new(canvas);
    //}
    pub fn render(&self, canvas: &mut skia_safe::Canvas) {
        let mut piet_ctx = Piet::new(canvas);
        let mut win_handler = borrow_mut!(self.handler).unwrap();
        win_handler.paint(&mut piet_ctx, &*borrow!(borrow!(self.window_state).unwrap().invalid).unwrap());
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

    pub fn connect(&self, handle: WindowHandle) {
        self.with_handler_and_dont_check_the_other_borrows(|h| {
            h.connect(&handle.into());
            h.scale(Scale::default());
        });
    }
}

/// Builder abstraction for creating new windows.
pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    cursor: Cursor,
    menu: Option<Menu>,
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<Window>);

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
    invalid: RefCell<Region>,
}

// TODO: support custom cursors
#[derive(Clone)]
pub struct CustomCursor;

impl WindowState {
    fn render(&self) {
        dbg!();
    }

    fn process_idle_queue(&self) {
        dbg!();
    }

    fn request_animation_frame(&self, f: impl FnOnce() + 'static) -> Result<i32, Error> {
        unimplemented!();
    }

    /// Returns the window size in css units
    fn get_window_size_and_dpr(&self) -> (f64, f64, f64) {
        unimplemented!()
    }

    /// Updates the canvas size and scale factor and returns `Scale` and `ScaledArea`.
    fn update_scale_and_area(&self) -> (Scale, ScaledArea) {
        unimplemented!()
    }
}


impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
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

    pub fn set_level(&mut self, _level: WindowLevel) {
        // ignored
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        let handler = self.handler.unwrap();
        // TODO
        let state = WindowState {
            scale: Cell::new(Scale::default()),
            area: Cell::new(ScaledArea::default()),
            idle_queue: Default::default(),
            invalid: RefCell::new(Region::EMPTY),
        };
        let window = Rc::new(Window {
            handler: RefCell::new(handler),
            window_state: RefCell::new(state),
        });
        
        let handle = WindowHandle(Rc::downgrade(&window));
        window.connect(handle.clone());
        self.app.add_window(window).unwrap(); // TODO Vlad handle error here
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

    pub fn set_level(&self, _level: WindowLevel) {
        log::warn!("WindowHandle::set_level  is currently unimplemented for web.");
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
        unimplemented!(); 
        //if let Some(s) = self.0.upgrade() {
        //    s.invalid.borrow_mut().add_rect(rect);
        //}
        //self.render_soon();
    }

    pub fn invalidate(&self) { 
        //unimplemented!(); 
        
        if let Some(s) = self.0.upgrade() {
            let s = borrow_mut!(s.window_state).unwrap();
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

        PietText::new()
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        unimplemented!()
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
    }

    pub fn make_cursor(&self, _cursor_desc: &CursorDesc) -> Option<Cursor> {
        log::warn!("Custom cursors are not yet supported in the web backend");
        None
    }

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        log::warn!("open_file is currently unimplemented for web.");
        None
    }

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        log::warn!("save_as is currently unimplemented for web.");
        None
    }

    fn render_soon(&self) { 
        log::warn!("VLAD TODO unimplemented")
        //if let Some(s) = self.0.upgrade() {
        //    let state = s.clone();
        //    s.request_animation_frame(move || {
        //        state.render();
        //    })
        //    .expect("Failed to request animation frame");
        //}
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
        log::warn!("VLAD implement idle_handle");
        None
        //unimplemented!(); 
        //self.0.upgrade().map(|w| IdleHandle {
        //    state: Rc::downgrade(&w),
        //    queue: w.idle_queue.clone(),
        //})
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> { 
        unimplemented!(); 
        //Ok(self
        //    .0
        //    .upgrade()
        //    .ok_or(ShellError::WindowDropped)?
        //    .scale
        //    .get())
    }

    pub fn set_menu(&self, _menu: Menu) {
        log::warn!("set_menu unimplemented for web");
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        log::warn!("show_context_menu unimplemented for web");
    }

    pub fn set_title(&self, title: impl Into<String>) {
        unimplemented!();
        //log::warn!("set_title is not implemented");
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
