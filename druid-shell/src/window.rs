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

//! Platform independent window types.

#![allow(deprecated)] // for the three items that have moved

use std::any::Any;

use crate::dialog::{FileDialogOptions, FileInfo};
use crate::error::Error;
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::kurbo::{Point, Size, Vec2};
use crate::menu::Menu;
use crate::mouse::{Cursor, MouseEvent};
use crate::platform::window as platform;

// It's possible we'll want to make this type alias at a lower level,
// see https://github.com/linebender/piet/pull/37 for more discussion.
/// The platform text factory, reexported from piet.
pub type Text<'a> = <piet_common::Piet<'a> as piet_common::RenderContext>::Text;

/// A token that uniquely identifies a running timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct TimerToken(usize);

impl TimerToken {
    /// A token that does not correspond to any timer.
    pub const INVALID: TimerToken = TimerToken(0);

    pub(crate) const fn new(id: usize) -> TimerToken {
        TimerToken(id)
    }

    #[cfg(target_os = "windows")]
    pub(crate) const fn get_raw(self) -> usize {
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
        self.0.add_idle(callback)
    }
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

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        self.0.bring_to_front_and_focus()
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        self.0.invalidate()
    }

    /// Set the title for this menu.
    pub fn set_title(&self, title: &str) {
        self.0.set_title(title)
    }

    /// Set the top-level menu for this window.
    pub fn set_menu(&self, menu: Menu) {
        self.0.set_menu(menu.into_inner())
    }

    /// Display a pop-up menu at the given position.
    ///
    /// `Point` is in the coordinate space of the window.
    pub fn show_context_menu(&self, menu: Menu, pos: Point) {
        self.0.show_context_menu(menu.into_inner(), pos)
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.0.get_idle_handle().map(IdleHandle)
    }

    /// Get the dpi of the window.
    ///
    /// TODO: we want to migrate this from dpi (with 96 as nominal) to a scale
    /// factor (with 1 as nominal).
    pub fn get_dpi(&self) -> f32 {
        self.0.get_dpi()
    }
}

/// A builder type for creating new windows.
pub struct WindowBuilder(platform::WindowBuilder);

impl WindowBuilder {
    /// Create a new `WindowBuilder`
    pub fn new() -> WindowBuilder {
        WindowBuilder(platform::WindowBuilder::new())
    }

    /// Set the [`WinHandler`]. This is the object that will receive
    /// callbacks from this window.
    ///
    /// [`WinHandler`]: trait.WinHandler.html
    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.0.set_handler(handler)
    }

    /// Set the window's initial size.
    pub fn set_size(&mut self, size: Size) {
        self.0.set_size(size)
    }

    /// Set the window's initial title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.0.set_title(title)
    }

    /// Set the window's menu.
    pub fn set_menu(&mut self, menu: Menu) {
        self.0.set_menu(menu.into_inner())
    }

    /// Attempt to construct the platform window.
    ///
    /// If this fails, your application should exit.
    pub fn build(self) -> Result<WindowHandle, Error> {
        self.0.build().map(WindowHandle).map_err(Into::into)
    }
}

/// A context supplied to most `WinHandler` methods.
pub trait WinCtx<'a> {
    /// Invalidate the entire window.
    ///
    /// TODO: finer grained invalidation.
    fn invalidate(&mut self);

    /// Get a reference to an object that can do text layout.
    fn text_factory(&mut self) -> &mut Text<'a>;

    /// Set the cursor icon.
    fn set_cursor(&mut self, cursor: &Cursor);

    /// Schedule a timer.
    ///
    /// This causes a [`WinHandler::timer()`] call at the deadline. The
    /// return value is a token that can be used to associate the request
    /// with the handler call.
    ///
    /// Note that this is not a precise timer. On Windows, the typical
    /// resolution is around 10ms. Therefore, it's best used for things
    /// like blinking a cursor or triggering tooltips, not for anything
    /// requiring precision.
    ///
    /// [`WinHandler::timer()`]: trait.WinHandler.html#tymethod.timer
    fn request_timer(&mut self, deadline: std::time::Instant) -> TimerToken;

    /// Prompt the user to chose a file to open.
    ///
    /// Blocks while the user picks the file.
    fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo>;

    /// Prompt the user to chose a path for saving.
    ///
    /// Blocks while the user picks a file.
    fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo>;
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

    /// Called immediately after `connect`.
    ///
    /// The handler can implement this method to perform initial setup.
    #[allow(unused_variables)]
    fn connected(&mut self, ctx: &mut dyn WinCtx) {}

    /// Called when the size of the window is changed. Note that size
    /// is in physical pixels.
    #[allow(unused_variables)]
    fn size(&mut self, width: u32, height: u32, ctx: &mut dyn WinCtx) {}

    /// Request the handler to paint the window contents. Return value
    /// indicates whether window is animating, i.e. whether another paint
    /// should be scheduled for the next animation frame.
    fn paint(&mut self, piet: &mut piet_common::Piet, ctx: &mut dyn WinCtx) -> bool;

    /// Called when the resources need to be rebuilt.
    ///
    /// Discussion: this function is mostly motivated by using
    /// `GenericRenderTarget` on Direct2D. If we move to `DeviceContext`
    /// instead, then it's possible we don't need this.
    #[allow(unused_variables)]
    fn rebuild_resources(&mut self, ctx: &mut dyn WinCtx) {}

    /// Called when a menu item is selected.
    #[allow(unused_variables)]
    fn command(&mut self, id: u32, ctx: &mut dyn WinCtx) {}

    /// Called on a key down event.
    ///
    /// Return `true` if the event is handled.
    #[allow(unused_variables)]
    fn key_down(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) -> bool {
        false
    }

    /// Called when a key is released. This corresponds to the WM_KEYUP message
    /// on Windows, or keyUp(withEvent:) on macOS.
    #[allow(unused_variables)]
    fn key_up(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) {}

    /// Called on a mouse wheel event.
    ///
    /// The polarity is the amount to be added to the scroll position,
    /// in other words the opposite of the direction the content should
    /// move on scrolling. This polarity is consistent with the
    /// deltaX and deltaY values in a web [WheelEvent].
    ///
    /// [WheelEvent]: https://w3c.github.io/uievents/#event-type-wheel
    #[allow(unused_variables)]
    fn wheel(&mut self, delta: Vec2, mods: KeyModifiers, ctx: &mut dyn WinCtx) {}

    /// Called when the mouse moves.
    #[allow(unused_variables)]
    fn mouse_move(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {}

    /// Called on mouse button down.
    #[allow(unused_variables)]
    fn mouse_down(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {}

    /// Called on mouse button up.
    #[allow(unused_variables)]
    fn mouse_up(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {}

    /// Called on timer event.
    ///
    /// This is called at (approximately) the requested deadline by a
    /// [`WinCtx::request_timer()`] call. The token argument here is the same
    /// as the return value of that call.
    ///
    /// [`WinCtx::request_timer()`]: trait.WinCtx.html#tymethod.request_timer
    #[allow(unused_variables)]
    fn timer(&mut self, token: TimerToken, ctx: &mut dyn WinCtx) {}

    /// Called when this window becomes the focused window.
    #[allow(unused_variables)]
    fn got_focus(&mut self, ctx: &mut dyn WinCtx) {}

    /// Called when the window is being destroyed. Note that this happens
    /// earlier in the sequence than drop (at WM_DESTROY, while the latter is
    /// WM_NCDESTROY).
    #[allow(unused_variables)]
    fn destroy(&mut self, ctx: &mut dyn WinCtx) {}

    /// Get a reference to the handler state. Used mostly by idle handlers.
    fn as_any(&mut self) -> &mut dyn Any;
}

impl From<platform::WindowHandle> for WindowHandle {
    fn from(src: platform::WindowHandle) -> WindowHandle {
        WindowHandle(src)
    }
}
