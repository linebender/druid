use std::any::Any;

use crate::window::{IdleToken, Text, TimerToken, WinHandler};
use crate::mouse::{Cursor};
use crate::dialog::{FileDialogOptions, FileInfo};
use crate::kurbo::{Point, Size};

use super::menu::Menu;
use super::error::Error;

pub struct WindowBuilder;

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        // TODO
        WindowBuilder {}
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        // TODO: currently a no-op
    }

    pub fn set_size(&mut self, size: Size) {
        unimplemented!(); // TODO
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        // TODO: currently a no-op
    }

    pub fn set_menu(&mut self, menu: Menu) {
        unimplemented!(); // TODO
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        // TODO: actual implementation
        Ok(WindowHandle::default())
    }
}

#[derive(Clone)]
pub struct IdleHandle;

impl IdleHandle {
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        unimplemented!(); // TODO
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        unimplemented!(); // TODO
    }
}

#[derive(Clone, Default)]
pub struct WindowHandle;

impl WindowHandle {
    pub fn show(&self) {
        // TODO: currently a no-op
    }

    pub fn close(&self) {
        unimplemented!(); // TODO
    }

    pub fn bring_to_front_and_focus(&self) {
        unimplemented!(); // TODO
    }

    pub fn invalidate(&self) {
        unimplemented!(); // TODO
    }

    pub fn set_title(&self, title: &str) {
        unimplemented!(); // TODO
    }

    pub fn set_menu(&self, menu: Menu) {
        unimplemented!(); // TODO
    }

    pub fn text(&self) -> Text {
        unimplemented!(); // TODO
    }

    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        unimplemented!(); // TODO
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        unimplemented!(); // TODO
    }

    pub fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        unimplemented!(); // TODO
    }

    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        unimplemented!(); // TODO
    }

    pub fn show_context_menu(&self, menu: Menu, pos: Point) {
        unimplemented!(); // TODO
    }

    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        unimplemented!(); // TODO
    }

    pub fn get_dpi(&self) -> f32 {
        unimplemented!(); // TODO
    }
}