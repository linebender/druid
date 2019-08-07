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

// It's possible we'll want to make this type alias at a lower level,
// see https://github.com/linebender/piet/pull/37 for more discussion.
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
    fn connect(&mut self, handle: &WindowHandle);

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

    /// Called when the window is being destroyed. Note that this happens
    /// earlier in the sequence than drop (at WM_DESTROY, while the latter is
    /// WM_NCDESTROY).
    #[allow(unused_variables)]
    fn destroy(&mut self, ctx: &mut dyn WinCtx) {}

    /// Get a reference to the handler state. Used mostly by idle handlers.
    fn as_any(&mut self) -> &mut dyn Any;
}

/// The state of the mouse for a click, mouse-up, or move event.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    /// The location of the mouse in the current window.
    ///
    /// This is in px units, that is, adjusted for hi-dpi.
    pub pos: Point,
    /// Keyboard modifiers at the time of the mouse event.
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
#[derive(Clone)]
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
