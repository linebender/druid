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

use std::any::Any;
use std::ops::Deref;

use crate::platform;

// Handle to Window Level Utilities
#[derive(Clone, Default)]
pub struct WindowHandle {
    pub inner: platform::WindowHandle,
}

impl Deref for WindowHandle {
    type Target = platform::WindowHandle;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
    fn connect(&self, handle: &WindowHandle);

    /// Called when the size of the window is changed. Note that size
    /// is in physical pixels.
    #[allow(unused_variables)]
    fn size(&self, width: u32, height: u32) {}

    /// Request the handler to paint the window contents. Return value
    /// indicates whether window is animating, i.e. whether another paint
    /// should be scheduled for the next animation frame.
    fn paint(&self, ctx: &mut piet_common::Piet) -> bool;

    /// Called when the resources need to be rebuilt.
    fn rebuild_resources(&self) {}

    #[allow(unused_variables)]
    /// Called when a menu item is selected.
    fn command(&self, id: u32) {}

    /// Called on keyboard input of a single character. This corresponds
    /// to the WM_CHAR message. Handling of text input will continue to
    /// evolve, we need to handle input methods and more.
    ///
    /// The modifiers are a combination of `M_ALT`, `M_CTRL`, `M_SHIFT`.
    #[allow(unused_variables)]
    fn char(&self, ch: u32, mods: u32) {}

    /// Called on a key down event. This corresponds to the WM_KEYDOWN
    /// message. The key code is as WM_KEYDOWN. We'll want to add stuff
    /// like the modifier state.
    ///
    /// The modifiers are a combination of `M_ALT`, `M_CTRL`, `M_SHIFT`.
    ///
    /// Return `true` if the event is handled.
    #[allow(unused_variables)]
    fn keydown(&self, vkey_code: i32, mods: u32) -> bool {
        false
    }

    /// Called on a mouse wheel event. This corresponds to a
    /// [WM_MOUSEWHEEL](https://msdn.microsoft.com/en-us/library/windows/desktop/ms645617(v=vs.85).aspx)
    /// message.
    ///
    /// The modifiers are the same as WM_MOUSEWHEEL.
    #[allow(unused_variables)]
    fn mouse_wheel(&self, delta: i32, mods: u32) {}

    /// Called on a mouse horizontal wheel event. This corresponds to a
    /// [WM_MOUSEHWHEEL](https://msdn.microsoft.com/en-us/library/windows/desktop/ms645614(v=vs.85).aspx)
    /// message.
    ///
    /// The modifiers are the same as WM_MOUSEHWHEEL.
    #[allow(unused_variables)]
    fn mouse_hwheel(&self, delta: i32, mods: u32) {}

    /// Called when the mouse moves. Note that the x, y coordinates are
    /// in absolute pixels.
    ///
    /// TODO: should we reuse the MouseEvent struct for this method as well?
    #[allow(unused_variables)]
    fn mouse_move(&self, x: i32, y: i32, mods: u32) {}

    /// Called on mouse button up or down. Note that the x, y
    /// coordinates are in absolute pixels.
    #[allow(unused_variables)]
    fn mouse(&self, event: &MouseEvent) {}

    /// Called when the window is being destroyed. Note that this happens
    /// earlier in the sequence than drop (at WM_DESTROY, while the latter is
    /// WM_NCDESTROY).
    fn destroy(&self) {}

    /// Get a reference to the handler state. Used mostly by idle handlers.
    fn as_any(&self) -> &Any;
}

/// A mouse button press or release event.
#[derive(Debug)]
pub struct MouseEvent {
    /// X coordinate in absolute pixels.
    pub x: i32,
    /// Y coordinate in absolute pixels.
    pub y: i32,
    /// Modifiers, as in raw WM message
    pub mods: u32,
    /// Which button was pressed or released.
    pub which: MouseButton,
    /// Type of event.
    pub ty: MouseType,
}

/// An indicator of which mouse button was pressed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Middle mouse button.
    Middle,
    /// Right mouse button.
    Right,
    /// First X button.
    X1,
    /// Second X button.
    X2,
}

/// An indicator of the state change of a mouse button.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MouseType {
    /// Mouse down event.
    Down,
    /// Note: DoubleClick is currently disabled, as we don't use the
    /// Windows processing.
    DoubleClick,
    /// Mouse up event.
    Up,
}

/// Standard cursor types. This is only a subset, others can be added as needed.
pub enum Cursor {
    Arrow,
    IBeam,
}
