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
use crate::kurbo::{Point, Size, Vec2};
use crate::pointer::{Button, Buttons, PointerEvent, PointerType};
use crate::scale::{Scalable, Scale};

use gtk::gdk::{self, AxisUse, ModifierType};

const MODIFIER_MAP: &[(gdk::ModifierType, Modifiers)] = &[
    (ModifierType::SHIFT_MASK, Modifiers::SHIFT),
    (ModifierType::MOD1_MASK, Modifiers::ALT),
    (ModifierType::CONTROL_MASK, Modifiers::CONTROL),
    (ModifierType::META_MASK, Modifiers::META),
    (ModifierType::LOCK_MASK, Modifiers::CAPS_LOCK),
    // Note: this is the usual value on X11, not sure how consistent it is.
    // Possibly we should use `Keymap::get_num_lock_state()` instead.
    (ModifierType::MOD2_MASK, Modifiers::NUM_LOCK),
];

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct PointerId {
    // This is an option because an event is apparently not guaranteed to come from a Device. All
    // such weird events will get assigned the id { None, None }.
    dev: Option<gdk::Device>,
    // Multi-touch devices assign unique ids to each sequence of events that is generated from a
    // single touch.
    seq: Option<gdk::EventSequence>,
}

fn get_modifiers(modifiers: gdk::ModifierType) -> Modifiers {
    let mut result = Modifiers::empty();
    for &(gdk_mod, modifier) in MODIFIER_MAP {
        if modifiers.contains(gdk_mod) {
            result |= modifier;
        }
    }
    result
}

fn get_button(button: u32) -> Option<Button> {
    match button {
        1 => Some(Button::Left),
        2 => Some(Button::Middle),
        3 => Some(Button::Right),
        // GDK X backend interprets button press events for button 4-7 as scroll events
        8 => Some(Button::X1),
        9 => Some(Button::X2),
        _ => None,
    }
}

fn get_buttons_from_modifiers(modifiers: gdk::ModifierType) -> Buttons {
    let mut buttons = Buttons::new();
    if modifiers.contains(ModifierType::BUTTON1_MASK) {
        buttons.insert(Button::Left);
    }
    if modifiers.contains(ModifierType::BUTTON2_MASK) {
        buttons.insert(Button::Middle);
    }
    if modifiers.contains(ModifierType::BUTTON3_MASK) {
        buttons.insert(Button::Right);
    }
    // TODO: Determine X1/X2 state (do caching ourselves if needed)
    //       Checking for BUTTON4_MASK/BUTTON5_MASK does not work with GDK X,
    //       because those are wheel events instead.
    if modifiers.contains(ModifierType::BUTTON4_MASK) {
        buttons.insert(Button::X1);
    }
    if modifiers.contains(ModifierType::BUTTON5_MASK) {
        buttons.insert(Button::X2);
    }
    buttons
}

fn get_pointer_type_from_event(ev: &gdk::Event) -> PointerType {
    if let Some(dev) = ev.source_device() {
        match dev.source() {
            gdk::InputSource::Mouse => PointerType::Mouse,
            gdk::InputSource::Pen => PointerType::Pen,
            gdk::InputSource::Eraser => PointerType::Eraser,
            gdk::InputSource::Touchscreen => PointerType::Touch,
            _ => PointerType::Mouse,
        }
    } else {
        PointerType::Mouse
    }
}

impl PointerEvent {
    // This is the "general purpose" conversion from gdk events to PointerEvents. For some specific
    // events, some of the PointerEvent fields might have better values.
    pub(crate) fn from_gdk(ev: &gdk::Event, scale: Scale) -> PointerEvent {
        let button_state = ev.state().unwrap_or_else(ModifierType::empty);
        let pressure = ev
            .axis(AxisUse::Pressure)
            .unwrap_or(if button_state.is_empty() { 0.0 } else { 0.5 });
        let tilt_x = ev.axis(AxisUse::Xtilt).unwrap_or(0.0);
        let tilt_y = ev.axis(AxisUse::Ytilt).unwrap_or(0.0);
        // FIXME: what are the units here?
        let twist = ev
            .axis(AxisUse::Rotation)
            .unwrap_or(0.0)
            .round()
            .clamp(0.0, 360.0) as u16;
        let button = get_button(ev.button().unwrap_or(0)).unwrap_or(Button::None);
        let id = PointerId {
            dev: ev.source_device(),
            seq: ev.event_sequence(),
        };
        PointerEvent {
            pointer_id: crate::PointerId(id),
            pos: Point::from(ev.coords().unwrap_or((0., 0.))).to_dp(scale),
            size: Size::new(1., 1.), // Seems to be unsupported in gtk?
            pressure,
            tangential_pressure: 0.0, // Seems to be unsupported in gtk?
            tilt_x,
            tilt_y,
            twist,
            mods: get_modifiers(button_state),
            button,
            buttons: get_buttons_from_modifiers(button_state).with(button),
            pointer_type: get_pointer_type_from_event(ev),
            // The remaining properties depend on the kind of event, so we just provide default
            // values here.
            is_primary: true,
            count: 0,
            focus: false,
            wheel_delta: Vec2::ZERO,
        }
    }
}
