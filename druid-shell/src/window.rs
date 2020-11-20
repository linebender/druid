// Copyright 2018 The Druid Authors.
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

//! Platform independent window types.

use std::any::Any;
use std::time::Duration;

use crate::application::Application;
use crate::common_util::Counter;
use crate::dialog::{FileDialogOptions, FileInfo};
use crate::error::Error;
use crate::keyboard::KeyEvent;
use crate::kurbo::{Point, Rect, Size};
use crate::menu::Menu;
use crate::mouse::{Cursor, CursorDesc, MouseEvent};
use crate::platform::window as platform;
use crate::region::Region;
use crate::scale::Scale;
use piet_common::PietText;

/// A token that uniquely identifies a running timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct TimerToken(u64);

impl TimerToken {
    /// A token that does not correspond to any timer.
    pub const INVALID: TimerToken = TimerToken(0);

    /// Create a new token.
    pub fn next() -> TimerToken {
        static TIMER_COUNTER: Counter = Counter::new();
        TimerToken(TIMER_COUNTER.next())
    }

    /// Create a new token from a raw value.
    pub const fn from_raw(id: u64) -> TimerToken {
        TimerToken(id)
    }

    /// Get the raw value for a token.
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

//NOTE: this has a From<platform::Handle> impl for construction
/// A handle that can enqueue tasks on the window loop.
#[derive(Clone)]
pub struct IdleHandle(platform::IdleHandle);

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
        self.0.add_idle_callback(callback)
    }

    /// Request a callback from the runloop. Your `WinHander::idle` method will
    /// be called with the `token` that was passed in.
    pub fn schedule_idle(&mut self, token: IdleToken) {
        self.0.add_idle_token(token)
    }
}

/// A token that uniquely identifies a idle schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct IdleToken(usize);

impl IdleToken {
    /// Create a new `IdleToken` with the given raw `usize` id.
    pub const fn new(raw: usize) -> IdleToken {
        IdleToken(raw)
    }
}

/// A token that uniquely identifies a file dialog request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct FileDialogToken(u64);

impl FileDialogToken {
    /// A token that does not correspond to any file dialog.
    pub const INVALID: FileDialogToken = FileDialogToken(0);

    /// Create a new token.
    pub fn next() -> FileDialogToken {
        static COUNTER: Counter = Counter::new();
        FileDialogToken(COUNTER.next())
    }

    /// Create a new token from a raw value.
    pub const fn from_raw(id: u64) -> FileDialogToken {
        FileDialogToken(id)
    }

    /// Get the raw value for a token.
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

/// Levels in the window system - Z order for display purposes.
/// Describes the purpose of a window and should be mapped appropriately to match platform
/// conventions.
#[derive(Copy, Clone, Debug)]
pub enum WindowLevel {
    /// A top level app window.
    AppWindow,
    /// A window that should stay above app windows - like a tooltip
    Tooltip,
    /// A user interface element such as a dropdown menu or combo box
    DropDown,
    /// A modal dialog
    Modal,
}

/// Contains the different states a Window can be in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowState {
    MAXIMIZED,
    MINIMIZED,
    RESTORED,
}

/// A handle to a platform window object.
#[derive(Clone, Default)]
pub struct WindowHandle(platform::WindowHandle);

impl WindowHandle {
    /// Make this window visible.
    ///
    /// This is part of the initialization process; it should only be called
    /// once, when a window is first created.
    pub fn show(&self) {
        self.0.show()
    }

    /// Close the window.
    pub fn close(&self) {
        self.0.close()
    }

    /// Set whether the window should be resizable
    pub fn resizable(&self, resizable: bool) {
        self.0.resizable(resizable)
    }

    /// Sets the state of the window.
    pub fn set_window_state(&mut self, state: WindowState) {
        self.0.set_window_state(state);
    }

    /// Gets the state of the window.
    pub fn get_window_state(&self) -> WindowState {
        self.0.get_window_state()
    }

    /// Informs the system that the current location of the mouse should be treated as part of the
    /// window's titlebar. This can be used to implement a custom titlebar widget. Note that
    /// because this refers to the current location of the mouse, you should probably call this
    /// function in response to every relevant [`WinHandler::mouse_move`].
    ///
    /// This is currently only implemented on Windows.
    pub fn handle_titlebar(&self, val: bool) {
        self.0.handle_titlebar(val);
    }

    /// Set whether the window should show titlebar.
    pub fn show_titlebar(&self, show_titlebar: bool) {
        self.0.show_titlebar(show_titlebar)
    }

    /// Sets the position of the window in [pixels](crate::Scale), relative to the origin of the
    /// virtual screen.
    pub fn set_position(&self, position: impl Into<Point>) {
        self.0.set_position(position.into())
    }

    /// Returns the position of the window in [pixels](crate::Scale), relative to the origin of the
    /// virtual screen.
    pub fn get_position(&self) -> Point {
        self.0.get_position()
    }

    /// Set the window's size in [display points](crate::Scale).
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.  The
    /// platform might choose a different size depending on its DPI or other platform-dependent
    /// configuration.  To know the actual size of the window you should handle the
    /// [`WinHandler::size`] method.
    pub fn set_size(&self, size: impl Into<Size>) {
        self.0.set_size(size.into())
    }

    /// Gets the window size, in [pixels](crate::Scale).
    pub fn get_size(&self) -> Size {
        self.0.get_size()
    }

    /// Sets the [`WindowLevel`](crate::WindowLevel), the z-order in the Window system / compositor
    ///
    /// We do not currently have a getter method, mostly because the system's levels aren't a
    /// perfect one-to-one map to `druid_shell`'s levels. A getter method may be added in the
    /// future.
    pub fn set_level(&self, level: WindowLevel) {
        self.0.set_level(level)
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        self.0.bring_to_front_and_focus()
    }

    /// Request that [`prepare_paint`] and [`paint`] be called next time there's the opportunity to
    /// render another frame. This differs from [`invalidate`] and [`invalidate_rect`] in that it
    /// doesn't invalidate any part of the window.
    ///
    /// [`invalidate`]: WindowHandle::invalidate
    /// [`invalidate_rect`]: WindowHandle::invalidate_rect
    /// [`paint`]: WinHandler::paint
    /// [`prepare_paint`]: WinHandler::prepare_paint
    pub fn request_anim_frame(&self) {
        self.0.request_anim_frame();
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        self.0.invalidate();
    }

    /// Request invalidation of a region of the window.
    pub fn invalidate_rect(&self, rect: Rect) {
        self.0.invalidate_rect(rect);
    }

    /// Set the title for this menu.
    pub fn set_title(&self, title: &str) {
        self.0.set_title(title)
    }

    /// Set the top-level menu for this window.
    pub fn set_menu(&self, menu: Menu) {
        self.0.set_menu(menu.into_inner())
    }

    /// Get access to a type that can perform text layout.
    pub fn text(&self) -> PietText {
        self.0.text()
    }

    /// Schedule a timer.
    ///
    /// This causes a [`WinHandler::timer`] call at the deadline. The
    /// return value is a token that can be used to associate the request
    /// with the handler call.
    ///
    /// Note that this is not a precise timer. On Windows, the typical
    /// resolution is around 10ms. Therefore, it's best used for things
    /// like blinking a cursor or triggering tooltips, not for anything
    /// requiring precision.
    pub fn request_timer(&self, deadline: Duration) -> TimerToken {
        self.0.request_timer(instant::Instant::now() + deadline)
    }

    /// Set the cursor icon.
    pub fn set_cursor(&mut self, cursor: &Cursor) {
        self.0.set_cursor(cursor)
    }

    pub fn make_cursor(&self, desc: &CursorDesc) -> Option<Cursor> {
        self.0.make_cursor(desc)
    }

    /// Prompt the user to choose a file to open.
    ///
    /// This won't block immediately; the file dialog will be shown whenever control returns to
    /// `druid-shell`, and the [`WinHandler::open_file`] method will be called when the dialog is
    /// closed.
    pub fn open_file(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        self.0.open_file(options)
    }

    /// Prompt the user to choose a path for saving.
    ///
    /// This won't block immediately; the file dialog will be shown whenever control returns to
    /// `druid-shell`, and the [`WinHandler::save_as`] method will be called when the dialog is
    /// closed.
    pub fn save_as(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        self.0.save_as(options)
    }

    /// Display a pop-up menu at the given position.
    ///
    /// `pos` is in the coordinate space of the window.
    pub fn show_context_menu(&self, menu: Menu, pos: Point) {
        self.0.show_context_menu(menu.into_inner(), pos)
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.0.get_idle_handle().map(IdleHandle)
    }

    /// Get the DPI scale of the window.
    ///
    /// The returned [`Scale`](crate::Scale) is a copy and thus its information will be stale after
    /// the platform DPI changes. This means you should not stash it and rely on it later; it is
    /// only guaranteed to be valid for the current pass of the runloop.
    pub fn get_scale(&self) -> Result<Scale, Error> {
        self.0.get_scale().map_err(Into::into)
    }
}

/// A builder type for creating new windows.
pub struct WindowBuilder(platform::WindowBuilder);

impl WindowBuilder {
    /// Create a new `WindowBuilder`.
    ///
    /// Takes the [`Application`](crate::Application) that this window is for.
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder(platform::WindowBuilder::new(app.platform_app))
    }

    /// Set the [`WinHandler`] for this window.
    ///
    /// This is the object that will receive callbacks from this window.
    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.0.set_handler(handler)
    }

    /// Set the window's initial drawing area size in [display points](crate::Scale).
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.  The
    /// platform might choose a different size depending on its DPI or other platform-dependent
    /// configuration.  To know the actual size of the window you should handle the
    /// [`WinHandler::size`] method.
    pub fn set_size(&mut self, size: Size) {
        self.0.set_size(size)
    }

    /// Set the window's minimum drawing area size in [display points](crate::Scale).
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    pub fn set_min_size(&mut self, size: Size) {
        self.0.set_min_size(size)
    }

    /// Set whether the window should be resizable.
    pub fn resizable(&mut self, resizable: bool) {
        self.0.resizable(resizable)
    }

    /// Set whether the window should have a titlebar and decorations.
    pub fn show_titlebar(&mut self, show_titlebar: bool) {
        self.0.show_titlebar(show_titlebar)
    }

    /// Sets the initial window position in [pixels](crate::Scale), relative to the origin of the
    /// virtual screen.
    pub fn set_position(&mut self, position: Point) {
        self.0.set_position(position);
    }

    /// Sets the initial [`WindowLevel`].
    pub fn set_level(&mut self, level: WindowLevel) {
        self.0.set_level(level);
    }

    /// Set the window's initial title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.0.set_title(title)
    }

    /// Set the window's menu.
    pub fn set_menu(&mut self, menu: Menu) {
        self.0.set_menu(menu.into_inner())
    }

    /// Sets the initial state of the window.
    pub fn set_window_state(&mut self, state: WindowState) {
        self.0.set_window_state(state);
    }

    /// Attempt to construct the platform window.
    ///
    /// If this fails, your application should exit.
    pub fn build(self) -> Result<WindowHandle, Error> {
        self.0.build().map(WindowHandle).map_err(Into::into)
    }
}

/// App behavior, supplied by the app.
///
/// Many of the "window procedure" messages map to calls to this trait.
/// The methods are non-mut because the window procedure can be called
/// recursively; implementers are expected to use `RefCell` or the like,
/// but should be careful to keep the lifetime of the borrow short.
pub trait WinHandler {
    /// Provide the handler with a handle to the window so that it can
    /// invalidate or make other requests.
    ///
    /// This method passes the `WindowHandle` directly, because the handler may
    /// wish to stash it.
    fn connect(&mut self, handle: &WindowHandle);

    /// Called when the size of the window has changed.
    ///
    /// The `size` parameter is the new size in [display points](crate::Scale).
    #[allow(unused_variables)]
    fn size(&mut self, size: Size) {}

    /// Called when the [scale](crate::Scale) of the window has changed.
    ///
    /// This is always called before the accompanying [`size`](WinHandler::size).
    #[allow(unused_variables)]
    fn scale(&mut self, scale: Scale) {}

    /// Request the handler to prepare to paint the window contents.  In particular, if there are
    /// any regions that need to be repainted on the next call to `paint`, the handler should
    /// invalidate those regions by calling [`WindowHandle::invalidate_rect`] or
    /// [`WindowHandle::invalidate`].
    fn prepare_paint(&mut self);

    /// Request the handler to paint the window contents.  `invalid` is the region in [display
    /// points](crate::Scale) that needs to be repainted; painting outside the invalid region will
    /// have no effect.
    fn paint(&mut self, piet: &mut piet_common::Piet, invalid: &Region);

    /// Called when the resources need to be rebuilt.
    ///
    /// Discussion: this function is mostly motivated by using
    /// `GenericRenderTarget` on Direct2D. If we move to `DeviceContext`
    /// instead, then it's possible we don't need this.
    #[allow(unused_variables)]
    fn rebuild_resources(&mut self) {}

    /// Called when a menu item is selected.
    #[allow(unused_variables)]
    fn command(&mut self, id: u32) {}

    /// Called when a "Save As" dialog is closed.
    ///
    /// `token` is the value returned by [`WindowHandle::save_as`]. `file` contains the information
    /// of the chosen path, or `None` if the save dialog was cancelled.
    #[allow(unused_variables)]
    fn save_as(&mut self, token: FileDialogToken, file: Option<FileInfo>) {}

    /// Called when an "Open" dialog is closed.
    ///
    /// `token` is the value returned by [`WindowHandle::open_file`]. `file` contains the information
    /// of the chosen path, or `None` if the save dialog was cancelled.
    #[allow(unused_variables)]
    fn open_file(&mut self, token: FileDialogToken, file: Option<FileInfo>) {}

    /// Called on a key down event.
    ///
    /// Return `true` if the event is handled.
    #[allow(unused_variables)]
    fn key_down(&mut self, event: KeyEvent) -> bool {
        false
    }

    /// Called when a key is released. This corresponds to the WM_KEYUP message
    /// on Windows, or keyUp(withEvent:) on macOS.
    #[allow(unused_variables)]
    fn key_up(&mut self, event: KeyEvent) {}

    /// Called on a mouse wheel event.
    ///
    /// The polarity is the amount to be added to the scroll position,
    /// in other words the opposite of the direction the content should
    /// move on scrolling. This polarity is consistent with the
    /// deltaX and deltaY values in a web [WheelEvent].
    ///
    /// [WheelEvent]: https://w3c.github.io/uievents/#event-type-wheel
    #[allow(unused_variables)]
    fn wheel(&mut self, event: &MouseEvent) {}

    /// Called when a platform-defined zoom gesture occurs (such as pinching
    /// on the trackpad).
    #[allow(unused_variables)]
    fn zoom(&mut self, delta: f64) {}

    /// Called when the mouse moves.
    #[allow(unused_variables)]
    fn mouse_move(&mut self, event: &MouseEvent) {}

    /// Called on mouse button down.
    #[allow(unused_variables)]
    fn mouse_down(&mut self, event: &MouseEvent) {}

    /// Called on mouse button up.
    #[allow(unused_variables)]
    fn mouse_up(&mut self, event: &MouseEvent) {}

    /// Called when the mouse cursor has left the application window
    fn mouse_leave(&mut self) {}

    /// Called on timer event.
    ///
    /// This is called at (approximately) the requested deadline by a
    /// [`WindowHandle::request_timer()`] call. The token argument here is the same
    /// as the return value of that call.
    #[allow(unused_variables)]
    fn timer(&mut self, token: TimerToken) {}

    /// Called when this window becomes the focused window.
    #[allow(unused_variables)]
    fn got_focus(&mut self) {}

    /// Called when this window stops being the focused window.
    #[allow(unused_variables)]
    fn lost_focus(&mut self) {}

    /// Called when the shell requests to close the window, for example because the user clicked
    /// the little "X" in the titlebar.
    ///
    /// If you want to actually close the window in response to this request, call
    /// [`WindowHandle::close`]. If you don't implement this method, clicking the titlebar "X" will
    /// have no effect.
    fn request_close(&mut self) {}

    /// Called when the window is being destroyed. Note that this happens
    /// earlier in the sequence than drop (at WM_DESTROY, while the latter is
    /// WM_NCDESTROY).
    #[allow(unused_variables)]
    fn destroy(&mut self) {}

    /// Called when a idle token is requested by [`IdleHandle::schedule_idle()`] call.
    #[allow(unused_variables)]
    fn idle(&mut self, token: IdleToken) {}

    /// Get a reference to the handler state. Used mostly by idle handlers.
    fn as_any(&mut self) -> &mut dyn Any;
}

impl From<platform::WindowHandle> for WindowHandle {
    fn from(src: platform::WindowHandle) -> WindowHandle {
        WindowHandle(src)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use static_assertions as sa;

    sa::assert_not_impl_any!(WindowHandle: Send, Sync);
    sa::assert_impl_all!(IdleHandle: Send);
}
