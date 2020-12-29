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

#[derive(Clone, Default)]
pub struct WindowHandle;

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
pub struct IdleHandle;

#[derive(Clone, PartialEq)]
pub struct CustomCursor;

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
        todo!()
    }
}

impl WindowHandle {
    pub fn show(&self) {
        todo!()
    }

    pub fn resizable(&self, resizable: bool) {
        todo!()
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        todo!()
    }

    pub fn set_position(&self, position: Point) {
        todo!()
    }

    pub fn get_position(&self) -> Point {
        todo!()
    }

    pub fn set_level(&self, level: WindowLevel) {
        todo!()
    }

    pub fn set_size(&self, size: Size) {
        todo!()
    }

    pub fn get_size(&self) -> Size {
        todo!()
    }

    pub fn set_window_state(&mut self, size_state: window::WindowState) {
        todo!()
    }

    pub fn get_window_state(&self) -> window::WindowState {
        todo!()
    }

    pub fn handle_titlebar(&self, _val: bool) {
        todo!()
    }

    /// Close the window.
    pub fn close(&self) {
        todo!()
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        todo!()
    }

    /// Request a new paint, but without invalidating anything.
    pub fn request_anim_frame(&self) {
        todo!()
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        todo!()
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    pub fn invalidate_rect(&self, rect: Rect) {
        todo!()
    }

    pub fn text(&self) -> PietText {
        todo!()
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        todo!()
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        todo!()
    }

    pub fn make_cursor(&self, desc: &CursorDesc) -> Option<Cursor> {
        todo!()
    }

    pub fn open_file(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        todo!()
    }

    pub fn save_as(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        todo!()
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        todo!()
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        todo!()
    }

    pub fn set_menu(&self, menu: Menu) {
        todo!()
    }

    pub fn show_context_menu(&self, menu: Menu, _pos: Point) {
        todo!()
    }

    pub fn set_title(&self, title: impl Into<String>) {
        todo!()
    }
}

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
        todo!()
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        todo!()
    }
}
