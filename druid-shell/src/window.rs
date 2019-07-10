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

pub use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::kurbo::{Point, Vec2};
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

    /// Called on a key down event.
    ///
    /// Return `true` if the event is handled.
    #[allow(unused_variables)]
    fn key_down(&self, event: KeyEvent) -> bool {
        false
    }

    /// Called when a key is released. This corresponds to the WM_KEYUP message
    /// on Windows, or keyUp(withEvent:) on macOS.
    #[allow(unused_variables)]
    fn key_up(&self, event: KeyEvent) {}

    /// Called on a mouse wheel event.
    ///
    /// The polarity is the amount to be added to the scroll position,
    /// in other words the opposite of the direction the content should
    /// move on scrolling. This polarity is consistent with the
    /// deltaX and deltaY values in a web [WheelEvent].
    ///
    /// [WheelEvent]: https://w3c.github.io/uievents/#event-type-wheel
    #[allow(unused_variables)]
    fn wheel(&self, delta: Vec2, mods: KeyModifiers) {}

    /// Called when the mouse moves.
    #[allow(unused_variables)]
    fn mouse_move(&self, event: &MouseEvent) {}

    /// Called on mouse button down.
    #[allow(unused_variables)]
    fn mouse_down(&self, event: &MouseEvent) {}

    /// Called on mouse button up.
    #[allow(unused_variables)]
    fn mouse_up(&self, event: &MouseEvent) {}

    /// Called when the window is being destroyed. Note that this happens
    /// earlier in the sequence than drop (at WM_DESTROY, while the latter is
    /// WM_NCDESTROY).
    fn destroy(&self) {}

    /// Get a reference to the handler state. Used mostly by idle handlers.
    fn as_any(&self) -> &dyn Any;
}

/// The state of the mouse for a click, mouse-up, or move event.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    /// The location of the mouse in the current window.
    ///
    /// This is in px units, that is, adjusted for hDPI.
    pub pos: Point,
    /// Modifiers, as in raw WM message
    pub mods: KeyModifiers,
    /// The number of mouse clicks associated with this event. This will always
    /// be `0` for a mouse-up event.
    pub count: u32,
    /// The currently pressed button in the case of a move or click event,
    /// or the released button in the case of a mouse-up event.
    pub button: MouseButton,
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

//NOTE: this currently only contains cursors that are included by default on
//both Windows and macOS. We may want to provide polyfills for various additional cursors,
//and we will also want to add some mechanism for adding custom cursors.
/// Mouse cursors.
pub enum Cursor {
    /// The default arrow cursor.
    Arrow,
    /// A vertical I-beam, for indicating insertion points in text.
    IBeam,
    Crosshair,
    OpenHand,
    NotAllowed,
    ResizeLeftRight,
    ResizeUpDown,
}
