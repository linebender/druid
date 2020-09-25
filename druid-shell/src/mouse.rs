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

//! Common types for representing mouse events and state

use crate::kurbo::{Point, Vec2};

use crate::Modifiers;

/// Information about the mouse event.
///
/// Every mouse event can have a new position. There is no guarantee of
/// receiving a move event before another mouse event.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    /// The location of the mouse in [display points] in relation to the current window.
    ///
    /// [display points]: struct.Scale.html
    pub pos: Point,
    /// Mouse buttons being held down during a move or after a click event.
    /// Thus it will contain the `button` that triggered a mouse-down event,
    /// and it will not contain the `button` that triggered a mouse-up event.
    pub buttons: MouseButtons,
    /// Keyboard modifiers at the time of the event.
    pub mods: Modifiers,
    /// The number of mouse clicks associated with this event. This will always
    /// be `0` for a mouse-up and mouse-move events.
    pub count: u8,
    /// Focus is `true` on macOS when the mouse-down event (or its companion mouse-up event)
    /// with `MouseButton::Left` was the event that caused the window to gain focus.
    pub focus: bool,
    /// The button that was pressed down in the case of mouse-down,
    /// or the button that was released in the case of mouse-up.
    /// This will always be `MouseButton::None` in the case of mouse-move.
    pub button: MouseButton,
    /// The wheel movement.
    ///
    /// The polarity is the amount to be added to the scroll position,
    /// in other words the opposite of the direction the content should
    /// move on scrolling. This polarity is consistent with the
    /// deltaX and deltaY values in a web [WheelEvent].
    ///
    /// [WheelEvent]: https://w3c.github.io/uievents/#event-type-wheel
    pub wheel_delta: Vec2,
}

/// An indicator of which mouse button was pressed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum MouseButton {
    /// No mouse button.
    // MUST BE FIRST (== 0)
    None,
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
    /// First X button.
    X1,
    /// Second X button.
    X2,
}

impl MouseButton {
    /// Returns `true` if this is [`MouseButton::Left`].
    ///
    /// [`MouseButton::Left`]: #variant.Left
    #[inline]
    pub fn is_left(self) -> bool {
        self == MouseButton::Left
    }

    /// Returns `true` if this is [`MouseButton::Right`].
    ///
    /// [`MouseButton::Right`]: #variant.Right
    #[inline]
    pub fn is_right(self) -> bool {
        self == MouseButton::Right
    }

    /// Returns `true` if this is [`MouseButton::Middle`].
    ///
    /// [`MouseButton::Middle`]: #variant.Middle
    #[inline]
    pub fn is_middle(self) -> bool {
        self == MouseButton::Middle
    }

    /// Returns `true` if this is [`MouseButton::X1`].
    ///
    /// [`MouseButton::X1`]: #variant.X1
    #[inline]
    pub fn is_x1(self) -> bool {
        self == MouseButton::X1
    }

    /// Returns `true` if this is [`MouseButton::X2`].
    ///
    /// [`MouseButton::X2`]: #variant.X2
    #[inline]
    pub fn is_x2(self) -> bool {
        self == MouseButton::X2
    }
}

/// A set of [`MouseButton`]s.
///
/// [`MouseButton`]: enum.MouseButton.html
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct MouseButtons(u8);

impl MouseButtons {
    /// Create a new empty set.
    #[inline]
    pub fn new() -> MouseButtons {
        MouseButtons(0)
    }

    /// Add the `button` to the set.
    #[inline]
    pub fn insert(&mut self, button: MouseButton) {
        self.0 |= 1.min(button as u8) << button as u8;
    }

    /// Remove the `button` from the set.
    #[inline]
    pub fn remove(&mut self, button: MouseButton) {
        self.0 &= !(1.min(button as u8) << button as u8);
    }

    /// Builder-style method for adding the `button` to the set.
    #[inline]
    pub fn with(mut self, button: MouseButton) -> MouseButtons {
        self.0 |= 1.min(button as u8) << button as u8;
        self
    }

    /// Builder-style method for removing the `button` from the set.
    #[inline]
    pub fn without(mut self, button: MouseButton) -> MouseButtons {
        self.0 &= !(1.min(button as u8) << button as u8);
        self
    }

    /// Returns `true` if the `button` is in the set.
    #[inline]
    pub fn contains(self, button: MouseButton) -> bool {
        (self.0 & (1.min(button as u8) << button as u8)) != 0
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all the `buttons` are in the set.
    #[inline]
    pub fn is_superset(self, buttons: MouseButtons) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Returns `true` if [`MouseButton::Left`] is in the set.
    ///
    /// [`MouseButton::Left`]: enum.MouseButton.html#variant.Left
    #[inline]
    pub fn has_left(self) -> bool {
        self.contains(MouseButton::Left)
    }

    /// Returns `true` if [`MouseButton::Right`] is in the set.
    ///
    /// [`MouseButton::Right`]: enum.MouseButton.html#variant.Right
    #[inline]
    pub fn has_right(self) -> bool {
        self.contains(MouseButton::Right)
    }

    /// Returns `true` if [`MouseButton::Middle`] is in the set.
    ///
    /// [`MouseButton::Middle`]: enum.MouseButton.html#variant.Middle
    #[inline]
    pub fn has_middle(self) -> bool {
        self.contains(MouseButton::Middle)
    }

    /// Returns `true` if [`MouseButton::X1`] is in the set.
    ///
    /// [`MouseButton::X1`]: enum.MouseButton.html#variant.X1
    #[inline]
    pub fn has_x1(self) -> bool {
        self.contains(MouseButton::X1)
    }

    /// Returns `true` if [`MouseButton::X2`] is in the set.
    ///
    /// [`MouseButton::X2`]: enum.MouseButton.html#variant.X2
    #[inline]
    pub fn has_x2(self) -> bool {
        self.contains(MouseButton::X2)
    }

    /// Adds all the `buttons` to the set.
    pub fn extend(&mut self, buttons: MouseButtons) {
        self.0 |= buttons.0;
    }

    /// Returns a union of the values in `self` and `other`.
    #[inline]
    pub fn union(mut self, other: MouseButtons) -> MouseButtons {
        self.0 |= other.0;
        self
    }

    /// Clear the set.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

impl std::fmt::Debug for MouseButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MouseButtons({:05b})", self.0 >> 1)
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
