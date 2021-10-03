// Copyright 2021 The Druid Authors.
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

//! The pointy bits

use druid_shell::kurbo::Vec2;
use druid_shell::Cursor;

use crate::kurbo::{Point, Size};
use crate::{Data, Event, Modifiers, PointerButton, PointerButtons, PointerId, PointerType};

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
    /// The location of the pointer in [display points] relative to the receiving widget.
    ///
    /// [display points]: struct.Scale.html
    pub pos: Point,
    /// The position of the mouse in [display points] relative to the window.
    ///
    /// [display points]: struct.Scale.html
    pub window_pos: Point,
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
    pub button: PointerButton,
    /// Buttons being held down during this event. Thus it will contain the `button` that
    /// triggered a mouse-down event, and it will not contain the `button` that triggered a
    /// mouse-up event.
    pub buttons: PointerButtons,
    /// The type of device that caused this event.
    pub pointer_type: PointerType,
    /// Indicates whether this is the primary pointer of its type. For example, the first finger
    /// to come down in a multi-touch event is the primary one.
    pub is_primary: bool,
    /// Indicates whether this event should be treated as coming from the system's single emulated
    /// mouse. See [Event] for more details.
    ///
    /// [Event]: crate::Event#The emulated mouse pointer
    pub is_emulated_mouse: bool,
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

impl From<druid_shell::PointerEvent> for PointerEvent {
    fn from(src: druid_shell::PointerEvent) -> PointerEvent {
        PointerEvent {
            pointer_id: src.pointer_id,
            pos: src.pos,
            window_pos: src.pos,
            size: src.size,
            pressure: src.pressure,
            tangential_pressure: src.tangential_pressure,
            tilt_x: src.tilt_x,
            tilt_y: src.tilt_y,
            twist: src.twist,
            mods: src.mods,
            button: src.button,
            buttons: src.buttons,
            pointer_type: src.pointer_type,
            is_primary: src.is_primary,
            is_emulated_mouse: false,
            count: src.count,
            focus: src.focus,
            wheel_delta: src.wheel_delta,
        }
    }
}

impl PointerEvent {
    pub(crate) fn translate(&self, offset: Vec2) -> PointerEvent {
        let mut ev = self.clone();
        ev.pos += offset;
        ev
    }
}

#[derive(Default)]
pub(crate) struct ActivePointerState {
    pen_active: bool,
}

impl ActivePointerState {
    fn is_emulated_mouse(&self, ptr: PointerType) -> bool {
        match ptr {
            PointerType::Mouse => true,
            PointerType::Pen => true,
            PointerType::Touch => !self.pen_active,
            PointerType::Eraser => false,
        }
    }

    pub fn pointer_down(&mut self, event: &druid_shell::PointerEvent) -> Event {
        if event.is_primary && event.pointer_type == PointerType::Pen {
            self.pen_active = true;
        }
        let mut event = PointerEvent::from(event.clone());
        if event.is_primary && self.is_emulated_mouse(event.pointer_type) {
            event.is_emulated_mouse = true;
            Event::MouseDown(event)
        } else {
            event.is_emulated_mouse = false;
            Event::PointerDown(event)
        }
    }

    pub fn pointer_up(&mut self, event: &druid_shell::PointerEvent) -> Event {
        if event.is_primary && event.pointer_type == PointerType::Pen {
            self.pen_active = false;
        }
        let mut event = PointerEvent::from(event.clone());
        if event.is_primary && self.is_emulated_mouse(event.pointer_type) {
            event.is_emulated_mouse = true;
            Event::MouseUp(event)
        } else {
            event.is_emulated_mouse = false;
            Event::PointerUp(event)
        }
    }

    pub fn pointer_move(&mut self, event: &druid_shell::PointerEvent) -> Event {
        if event.is_primary && event.pointer_type == PointerType::Pen {
            self.pen_active = false;
        }
        let mut event = PointerEvent::from(event.clone());
        if event.is_primary && self.is_emulated_mouse(event.pointer_type) {
            event.is_emulated_mouse = true;
            Event::MouseMove(event)
        } else {
            event.is_emulated_mouse = false;
            Event::PointerMove(event)
        }
    }
}

impl Data for Cursor {
    fn same(&self, other: &Cursor) -> bool {
        self == other
    }
}
