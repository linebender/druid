// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Web keycode handling.

use web_sys::KeyboardEvent;

use crate::keyboard::{Code, KbKey, KeyEvent, KeyState, Location, Modifiers};

/// Convert a web-sys KeyboardEvent into a keyboard-types one.
pub(crate) fn convert_keyboard_event(
    event: &KeyboardEvent,
    mods: Modifiers,
    state: KeyState,
) -> KeyEvent {
    KeyEvent {
        state,
        key: event.key().parse().unwrap_or(KbKey::Unidentified),
        code: convert_code(&event.code()),
        location: convert_location(event.location()),
        mods,
        repeat: event.repeat(),
        is_composing: event.is_composing(),
    }
}

fn convert_code(code: &str) -> Code {
    code.parse().unwrap_or(Code::Unidentified)
}

fn convert_location(loc: u32) -> Location {
    match loc {
        KeyboardEvent::DOM_KEY_LOCATION_LEFT => Location::Left,
        KeyboardEvent::DOM_KEY_LOCATION_RIGHT => Location::Right,
        KeyboardEvent::DOM_KEY_LOCATION_NUMPAD => Location::Numpad,
        // Should be exhaustive but in case not, use reasonable default
        _ => Location::Standard,
    }
}
