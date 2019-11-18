// Copyright 2019 The xi-editor Authors.
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

//! Common types for representing mouse events and state

use crate::kurbo::Point;

use crate::keyboard::KeyModifiers;

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

impl MouseButton {
    /// Returns `true` if this is the left mouse button.
    #[inline(always)]
    pub fn is_left(self) -> bool {
        self == MouseButton::Left
    }

    /// Returns `true` if this is the right mouse button.
    #[inline(always)]
    pub fn is_right(self) -> bool {
        self == MouseButton::Right
    }
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
