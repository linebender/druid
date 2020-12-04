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

//! The mousey bits

use crate::kurbo::{Point, Vec2};
use crate::{Cursor, Data, Modifiers, MouseButton, MouseButtons};

/// The state of the mouse for a click, mouse-up, move, or wheel event.
///
/// In `druid`, unlike in `druid_shell`, we treat the widget's coordinate
/// space and the window's coordinate space separately.
///
/// Every mouse event can have a new position. There is no guarantee of
/// receiving an [`Event::MouseMove`] before another mouse event.
///
/// When comparing to the position that was reported by [`Event::MouseMove`],
/// the position in relation to the window might have changed because
/// the window moved or the platform just didn't inform us of the move.
/// The position may also have changed in relation to the receiver,
/// because the receiver's location changed without the mouse moving.
///
/// [`Event::MouseMove`]: enum.Event.html#variant.MouseMove
#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// The position of the mouse in the coordinate space of the receiver.
    pub pos: Point,
    /// The position of the mouse in the coordinate space of the window.
    pub window_pos: Point,
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
    ///
    /// This is primarily used in relation to text selection.
    /// If there is some text selected in some text widget and it receives a click
    /// with `focus` set to `true` then the widget should gain focus (i.e. start blinking a cursor)
    /// but it should not change the text selection. Text selection should only be changed
    /// when the click has `focus` set to `false`.
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

impl From<druid_shell::MouseEvent> for MouseEvent {
    fn from(src: druid_shell::MouseEvent) -> MouseEvent {
        let druid_shell::MouseEvent {
            pos,
            buttons,
            mods,
            count,
            focus,
            button,
            wheel_delta,
        } = src;
        MouseEvent {
            pos,
            window_pos: pos,
            buttons,
            mods,
            count,
            focus,
            button,
            wheel_delta,
        }
    }
}

impl Data for Cursor {
    fn same(&self, other: &Cursor) -> bool {
        self == other
    }
}
