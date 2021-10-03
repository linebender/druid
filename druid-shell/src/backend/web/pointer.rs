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

use crate::keyboard::Modifiers;
use crate::kurbo::Point;
use crate::pointer::{Button, Buttons, PointerEvent, PointerType};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct PointerId {}

fn mouse_buttons(mask: u16) -> Buttons {
    let mut buttons = Buttons::new();
    if mask & 1 != 0 {
        buttons.insert(Button::Left);
    }
    if mask & 1 << 1 != 0 {
        buttons.insert(Button::Right);
    }
    if mask & 1 << 2 != 0 {
        buttons.insert(Button::Middle);
    }
    if mask & 1 << 3 != 0 {
        buttons.insert(Button::X1);
    }
    if mask & 1 << 4 != 0 {
        buttons.insert(Button::X2);
    }
    buttons
}

impl PointerEvent {
    // TODO: support creating pointer events from web PointerEvents.
    pub(crate) fn from_web(event: &web_sys::MouseEvent) -> PointerEvent {
        let buttons = mouse_buttons(event.buttons());
        let pos = Point::new(event.offset_x() as f64, event.offset_y() as f64);
        PointerEvent {
            pointer_id: crate::PointerId(PointerId {}),
            pos,
            buttons,
            is_primary: true,
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
