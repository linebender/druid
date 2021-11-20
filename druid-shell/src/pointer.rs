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

//! Common types for representing pointer events and pointer state.
//!
//! This module is based on the [W3C Pointer Events recommendation].
//!
//! [W3C Pointer Events recommendation]: https://www.w3.org/TR/pointerevents/

use crate::backend;
use crate::kurbo::{Point, Size, Vec2};
use crate::Modifiers;

/// An unique identifier for a pointer.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PointerId(pub(crate) backend::pointer::PointerId);

/// An event caused by a pointer.
///
/// This unifies events caused by several different input devices, including mice, touch input, and
/// pen input.
///
/// Our pointer events are based on the [W3C specification].
///
/// [W3C specification]: https://w3c.github.io/pointerevents/
#[derive(Clone, Debug)]
pub struct PointerEvent {
    /// A unique identifier for the pointer that caused this event.
    ///
    /// These are not globally unique for all time, but a pointer id will not be reused as long as
    /// that pointer remains "active." For example, if you touch the screen with multiple different
    /// fingers at the same time, each finger will get a different id. But if you lift your fingers
    /// and then press them again, they might reuse some of the previous ids.
    pub pointer_id: PointerId,
    /// The location of the pointer in [display points] in relation to the current window.
    ///
    /// [display points]: struct.Scale.html
    pub pos: Point,
    /// The size of the pointer. Some pointers (like mice) have zero width and height. Others (like
    /// fingers) might have positive size.
    pub size: Size,
    /// The normalized pressure of the pointer input in the range `[0, 1]`, where `0` and `1`
    /// represent the minimum and maximum possible pressure that the hardware can detect,
    /// respectively. For hardware that doesn't support pressure, this value will either be `0.5`
    /// (when, e.g., the pen is touching the screen, or a mouse button is down) or `0.0` (when,
    /// e.g. the pen or mouse button is lifted).
    pub pressure: f64,
    /// The normalized tangential pressure, in the range `[-1, 1]`. For hardward that does not
    /// support tangential pressure, this will be `0`.
    pub tangential_pressure: f64,
    /// In degrees, the angle between the Y-Z plane and the plane containing the pen or stylus.
    /// This is in the range `[-90, 90]`, and it will be `0` for hardware that does not support
    /// tilt.
    pub tilt_x: f64,
    /// In degrees, the angle between the X-Z plane and the plane containing the pen or stylus.
    /// This is in the range `[-90, 90]`, and it will be `0` for hardware that does not support
    /// tilt.
    pub tilt_y: f64,
    /// The clockwise rotation of a pen or stylus around its own major axis. This is in the range
    /// `[0, 359]`, and it will be `0` for hardward that does not support twist.
    pub twist: u16,
    /// Keyboard modifiers at the time of the event.
    pub mods: Modifiers,
    /// The button whose state-change caused this event. When this event was caused by something
    /// other than a button press (for example, a pointer move), this will be `Button::None`.
    pub button: Button,
    /// Buttons being held down during this event. Thus it will contain the `button` that
    /// triggered a mouse-down event, and it will not contain the `button` that triggered a
    /// mouse-up event.
    pub buttons: Buttons,
    /// The type of device that caused this event.
    pub pointer_type: PointerType,
    /// Indicates whether this is the primary pointer of its type. For example, the first finger
    /// to come down in a multi-touch event is the primary one.
    pub is_primary: bool,
    /// The number of mouse clicks associated with this event. This will always
    /// be `0` for pointer-up and pointer-move events.
    pub count: u8,
    /// Focus is `true` on macOS when the mouse-down event (or its companion mouse-up event)
    /// with `MouseButton::Left` was the event that caused the window to gain focus.
    pub focus: bool,
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

/// The types of pointers that can cause events.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PointerType {
    Mouse,
    Touch,
    Pen,
    Eraser,
}

/// An indicator of which mouse button was pressed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Button {
    /// No button.
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

impl Button {
    /// Returns `true` if this is [`Button::Left`].
    ///
    /// [`Button::Left`]: #variant.Left
    #[inline]
    pub fn is_left(self) -> bool {
        self == Button::Left
    }

    /// Returns `true` if this is [`Button::Right`].
    ///
    /// [`Button::Right`]: #variant.Right
    #[inline]
    pub fn is_right(self) -> bool {
        self == Button::Right
    }

    /// Returns `true` if this is [`Button::Middle`].
    ///
    /// [`Button::Middle`]: #variant.Middle
    #[inline]
    pub fn is_middle(self) -> bool {
        self == Button::Middle
    }

    /// Returns `true` if this is [`Button::X1`].
    ///
    /// [`Button::X1`]: #variant.X1
    #[inline]
    pub fn is_x1(self) -> bool {
        self == Button::X1
    }

    /// Returns `true` if this is [`Button::X2`].
    ///
    /// [`Button::X2`]: #variant.X2
    #[inline]
    pub fn is_x2(self) -> bool {
        self == Button::X2
    }
}

/// A set of [`Button`]s.
///
/// [`Button`]: enum.Button.html
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct Buttons(u8);

impl Buttons {
    /// Create a new empty set.
    #[inline]
    pub fn new() -> Buttons {
        Buttons(0)
    }

    /// Add the `button` to the set.
    #[inline]
    pub fn insert(&mut self, button: Button) {
        self.0 |= 1.min(button as u8) << button as u8;
    }

    /// Remove the `button` from the set.
    #[inline]
    pub fn remove(&mut self, button: Button) {
        self.0 &= !(1.min(button as u8) << button as u8);
    }

    /// Builder-style method for adding the `button` to the set.
    #[inline]
    pub fn with(mut self, button: Button) -> Buttons {
        self.0 |= 1.min(button as u8) << button as u8;
        self
    }

    /// Builder-style method for removing the `button` from the set.
    #[inline]
    pub fn without(mut self, button: Button) -> Buttons {
        self.0 &= !(1.min(button as u8) << button as u8);
        self
    }

    /// Returns `true` if the `button` is in the set.
    #[inline]
    pub fn contains(self, button: Button) -> bool {
        (self.0 & (1.min(button as u8) << button as u8)) != 0
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all the `buttons` are in the set.
    #[inline]
    pub fn is_superset(self, buttons: Buttons) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Returns `true` if [`Button::Left`] is in the set.
    ///
    /// [`Button::Left`]: enum.Button.html#variant.Left
    #[inline]
    pub fn has_left(self) -> bool {
        self.contains(Button::Left)
    }

    /// Returns `true` if [`Button::Right`] is in the set.
    ///
    /// [`Button::Right`]: enum.Button.html#variant.Right
    #[inline]
    pub fn has_right(self) -> bool {
        self.contains(Button::Right)
    }

    /// Returns `true` if [`Button::Middle`] is in the set.
    ///
    /// [`Button::Middle`]: enum.Button.html#variant.Middle
    #[inline]
    pub fn has_middle(self) -> bool {
        self.contains(Button::Middle)
    }

    /// Returns `true` if [`Button::X1`] is in the set.
    ///
    /// [`Button::X1`]: enum.Button.html#variant.X1
    #[inline]
    pub fn has_x1(self) -> bool {
        self.contains(Button::X1)
    }

    /// Returns `true` if [`Button::X2`] is in the set.
    ///
    /// [`Button::X2`]: enum.Button.html#variant.X2
    #[inline]
    pub fn has_x2(self) -> bool {
        self.contains(Button::X2)
    }

    /// Adds all the `buttons` to the set.
    pub fn extend(&mut self, buttons: Buttons) {
        self.0 |= buttons.0;
    }

    /// Returns a union of the values in `self` and `other`.
    #[inline]
    pub fn union(mut self, other: Buttons) -> Buttons {
        self.0 |= other.0;
        self
    }

    /// Clear the set.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Count the number of pressed buttons in the set.
    #[inline]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
}

impl std::fmt::Debug for Buttons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Buttons({:05b})", self.0 >> 1)
    }
}
