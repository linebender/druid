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

use winapi::shared::minwindef::*;
use winapi::um::winuser::*;

use crate::keyboard::Modifiers;
use crate::kurbo::Point;
use crate::scale::Scalable;
use crate::{Button, Buttons, PointerEvent, PointerType, Scale};

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct PointerId(u16);

// These are copied/adapted from macros in WinUser.h that don't appear to be in winapi-rs.
fn is_primary(wparam: WPARAM) -> bool {
    HIWORD(wparam as u32) as u32 & POINTER_FLAG_PRIMARY != 0
}

fn id(wparam: WPARAM) -> crate::PointerId {
    crate::PointerId(PointerId(LOWORD(wparam as u32) as u16))
}

/// Extract the buttons that are being held down from wparam in mouse events.
fn get_buttons(wparam: WPARAM) -> Buttons {
    let mut buttons = Buttons::new();
    if wparam & MK_LBUTTON != 0 {
        buttons.insert(Button::Left);
    }
    if wparam & MK_RBUTTON != 0 {
        buttons.insert(Button::Right);
    }
    if wparam & MK_MBUTTON != 0 {
        buttons.insert(Button::Middle);
    }
    if wparam & MK_XBUTTON1 != 0 {
        buttons.insert(Button::X1);
    }
    if wparam & MK_XBUTTON2 != 0 {
        buttons.insert(Button::X2);
    }
    buttons
}

impl PointerEvent {
    pub(crate) fn from_win(wparam: WPARAM, lparam: LPARAM, scale: Scale) -> PointerEvent {
        let x = LOWORD(lparam as u32) as i16 as i32;
        let y = HIWORD(lparam as u32) as i16 as i32;
        let pos = Point::new(x as f64, y as f64).to_dp(scale);
        let buttons = get_buttons(wparam);
        PointerEvent {
            pointer_id: id(wparam),
            pos,
            buttons,
            is_primary: is_primary(wparam),
            // TODO: support these properties
            size: (1.0, 1.0).into(),
            pressure: 0.0,
            tangential_pressure: 0.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0,
            pointer_type: PointerType::Mouse,
            // The remaining properties depend on the kind of event, so we just provide default
            // values here.
            count: 0,
            focus: false,
            wheel_delta: (0.0, 0.0).into(),
            mods: Modifiers::empty(),
            button: Button::None,
        }
    }
}
