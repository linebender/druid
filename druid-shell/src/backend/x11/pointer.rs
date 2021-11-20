// Copyright 2020 The Druid Authors.
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

use kurbo::{Point, Size};
use tracing::warn;
use x11rb::protocol::xproto;

use crate::pointer::{Button, Buttons, PointerType};
use crate::scale::Scalable;
use crate::{PointerEvent, Scale};

use super::util::key_mods;

// TODO: support non-mouse pointers
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct PointerId {}

impl PointerEvent {
    fn from_state(state: u16, detail: u8) -> PointerEvent {
        PointerEvent {
            pointer_id: crate::PointerId(PointerId {}),
            pos: (0., 0.).into(),
            button: button(detail),
            buttons: buttons(state),
            mods: key_mods(state),
            // These fields should be supported, but aren't yet.
            pressure: 0.0,
            tangential_pressure: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0,
            size: Size::new(1., 1.),
            is_primary: true,
            pointer_type: PointerType::Mouse,

            // These fields aren't handled here, because they depend on other properties of the
            // event.
            count: 0,
            focus: false,
            wheel_delta: (0., 0.).into(),
        }
    }

    pub(crate) fn from_button_press(ev: &xproto::ButtonPressEvent, scale: Scale) -> PointerEvent {
        let mut ptr_ev = PointerEvent::from_state(ev.state, ev.detail);
        ptr_ev.pos = Point::new(ev.event_x as f64, ev.event_y as f64).to_dp(scale);
        // The xcb state field doesn't include the newly pressed button, but
        // druid wants it to be included.
        ptr_ev.buttons.insert(ptr_ev.button);
        ptr_ev
    }

    pub(crate) fn from_button_release(
        ev: &xproto::ButtonReleaseEvent,
        scale: Scale,
    ) -> PointerEvent {
        let mut ptr_ev = PointerEvent::from_state(ev.state, ev.detail);
        ptr_ev.pos = Point::new(ev.event_x as f64, ev.event_y as f64).to_dp(scale);
        // The xcb state includes the newly released button, but druid
        // doesn't want it.
        ptr_ev.buttons.remove(ptr_ev.button);
        ptr_ev
    }

    pub(crate) fn from_motion_notify(ev: &xproto::MotionNotifyEvent, scale: Scale) -> PointerEvent {
        let mut ptr_ev = PointerEvent::from_state(ev.state, 1);
        ptr_ev.button = Button::None;
        ptr_ev.pos = Point::new(ev.event_x as f64, ev.event_y as f64).to_dp(scale);
        ptr_ev
    }
}

// Converts from, e.g., the `details` field of `xcb::xproto::ButtonPressEvent`
fn button(button: u8) -> Button {
    match button {
        1 => Button::Left,
        2 => Button::Middle,
        3 => Button::Right,
        // buttons 4 through 7 are for scrolling.
        4..=7 => Button::None,
        8 => Button::X1,
        9 => Button::X2,
        _ => {
            warn!("unknown mouse button code {}", button);
            Button::None
        }
    }
}

// Extracts the mouse buttons from, e.g., the `state` field of
// `xcb::xproto::ButtonPressEvent`
fn buttons(mods: u16) -> Buttons {
    let mut buttons = Buttons::new();
    let button_masks = &[
        (xproto::ButtonMask::M1, Button::Left),
        (xproto::ButtonMask::M2, Button::Middle),
        (xproto::ButtonMask::M3, Button::Right),
        // TODO: determine the X1/X2 state, using our own caching if necessary.
        // BUTTON_MASK_4/5 do not work: they are for scroll events.
    ];
    for (mask, button) in button_masks {
        if mods & u16::from(*mask) != 0 {
            buttons.insert(*button);
        }
    }
    buttons
}
