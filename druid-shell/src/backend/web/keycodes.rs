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
