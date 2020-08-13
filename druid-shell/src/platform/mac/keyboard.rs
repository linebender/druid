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

//! Conversion of platform keyboard event into cross-platform event.

use cocoa::appkit::{NSEvent, NSEventModifierFlags, NSEventType};
use cocoa::base::id;
use objc::{msg_send, sel, sel_impl};

use crate::keyboard::{Code, KbKey, KeyEvent, KeyState, Modifiers};

use super::super::shared;
use super::util::from_nsstring;

/// State for processing of keyboard events.
///
/// This needs to be stateful for proper processing of dead keys. The current
/// implementation is somewhat primitive and is not based on IME; in the future
/// when IME is implemented, it will need to be redone somewhat, letting the IME
/// be the authoritative source of truth for Unicode string values of keys.
///
/// Most of the logic in this module is adapted from Mozilla, and in particular
/// TextInputHandler.mm.
pub(crate) struct KeyboardState {
    last_mods: NSEventModifierFlags,
}

/// Convert a macOS platform key code (keyCode field of NSEvent).
///
/// The primary source for this mapping is:
/// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code/code_values
///
/// It should also match up with CODE_MAP_MAC bindings in
/// NativeKeyToDOMCodeName.h.
fn key_code_to_code(key_code: u16) -> Code {
    match key_code {
        0x00 => Code::KeyA,
        0x01 => Code::KeyS,
        0x02 => Code::KeyD,
        0x03 => Code::KeyF,
        0x04 => Code::KeyH,
        0x05 => Code::KeyG,
        0x06 => Code::KeyZ,
        0x07 => Code::KeyX,
        0x08 => Code::KeyC,
        0x09 => Code::KeyV,
        0x0a => Code::IntlBackslash,
        0x0b => Code::KeyB,
        0x0c => Code::KeyQ,
        0x0d => Code::KeyW,
        0x0e => Code::KeyE,
        0x0f => Code::KeyR,
        0x10 => Code::KeyY,
        0x11 => Code::KeyT,
        0x12 => Code::Digit1,
        0x13 => Code::Digit2,
        0x14 => Code::Digit3,
        0x15 => Code::Digit4,
        0x16 => Code::Digit6,
        0x17 => Code::Digit5,
        0x18 => Code::Equal,
        0x19 => Code::Digit9,
        0x1a => Code::Digit7,
        0x1b => Code::Minus,
        0x1c => Code::Digit8,
        0x1d => Code::Digit0,
        0x1e => Code::BracketRight,
        0x1f => Code::KeyO,
        0x20 => Code::KeyU,
        0x21 => Code::BracketLeft,
        0x22 => Code::KeyI,
        0x23 => Code::KeyP,
        0x24 => Code::Enter,
        0x25 => Code::KeyL,
        0x26 => Code::KeyJ,
        0x27 => Code::Quote,
        0x28 => Code::KeyK,
        0x29 => Code::Semicolon,
        0x2a => Code::Backslash,
        0x2b => Code::Comma,
        0x2c => Code::Slash,
        0x2d => Code::KeyN,
        0x2e => Code::KeyM,
        0x2f => Code::Period,
        0x30 => Code::Tab,
        0x31 => Code::Space,
        0x32 => Code::Backquote,
        0x33 => Code::Backspace,
        0x34 => Code::NumpadEnter,
        0x35 => Code::Escape,
        0x36 => Code::MetaRight,
        0x37 => Code::MetaLeft,
        0x38 => Code::ShiftLeft,
        0x39 => Code::CapsLock,
        // Note: in the linked source doc, this is "OSLeft"
        0x3a => Code::AltLeft,
        0x3b => Code::ControlLeft,
        0x3c => Code::ShiftRight,
        // Note: in the linked source doc, this is "OSRight"
        0x3d => Code::AltRight,
        0x3e => Code::ControlRight,
        0x3f => Code::Fn, // No events fired
        //0x40 => Code::F17,
        0x41 => Code::NumpadDecimal,
        0x43 => Code::NumpadMultiply,
        0x45 => Code::NumpadAdd,
        0x47 => Code::NumLock,
        0x48 => Code::AudioVolumeUp,
        0x49 => Code::AudioVolumeDown,
        0x4a => Code::AudioVolumeMute,
        0x4b => Code::NumpadDivide,
        0x4c => Code::NumpadEnter,
        0x4e => Code::NumpadSubtract,
        //0x4f => Code::F18,
        //0x50 => Code::F19,
        0x51 => Code::NumpadEqual,
        0x52 => Code::Numpad0,
        0x53 => Code::Numpad1,
        0x54 => Code::Numpad2,
        0x55 => Code::Numpad3,
        0x56 => Code::Numpad4,
        0x57 => Code::Numpad5,
        0x58 => Code::Numpad6,
        0x59 => Code::Numpad7,
        //0x5a => Code::F20,
        0x5b => Code::Numpad8,
        0x5c => Code::Numpad9,
        0x5d => Code::IntlYen,
        0x5e => Code::IntlRo,
        0x5f => Code::NumpadComma,
        0x60 => Code::F5,
        0x61 => Code::F6,
        0x62 => Code::F7,
        0x63 => Code::F3,
        0x64 => Code::F8,
        0x65 => Code::F9,
        0x66 => Code::Lang2,
        0x67 => Code::F11,
        0x68 => Code::Lang1,
        // Note: this is listed as F13, but in testing with a standard
        // USB kb, this the code produced by PrtSc.
        0x69 => Code::PrintScreen,
        //0x6a => Code::F16,
        //0x6b => Code::F14,
        0x6d => Code::F10,
        0x6e => Code::ContextMenu,
        0x6f => Code::F12,
        //0x71 => Code::F15,
        0x72 => Code::Help,
        0x73 => Code::Home,
        0x74 => Code::PageUp,
        0x75 => Code::Delete,
        0x76 => Code::F4,
        0x77 => Code::End,
        0x78 => Code::F2,
        0x79 => Code::PageDown,
        0x7a => Code::F1,
        0x7b => Code::ArrowLeft,
        0x7c => Code::ArrowRight,
        0x7d => Code::ArrowDown,
        0x7e => Code::ArrowUp,
        _ => Code::Unidentified,
    }
}

/// Convert code to key.
///
/// On macOS, for non-printable keys, the keyCode we get from the event serves is
/// really more of a key than a physical scan code.
///
/// When this function returns None, the code can be considered printable.
///
/// The logic for this function is derived from KEY_MAP_COCOA bindings in
/// NativeKeyToDOMKeyName.h.
fn code_to_key(code: Code) -> Option<KbKey> {
    Some(match code {
        Code::Escape => KbKey::Escape,
        Code::ShiftLeft | Code::ShiftRight => KbKey::Shift,
        Code::AltLeft | Code::AltRight => KbKey::Alt,
        Code::MetaLeft | Code::MetaRight => KbKey::Meta,
        Code::ControlLeft | Code::ControlRight => KbKey::Control,
        Code::CapsLock => KbKey::CapsLock,
        // kVK_ANSI_KeypadClear
        Code::NumLock => KbKey::Clear,
        Code::Fn => KbKey::Fn,
        Code::F1 => KbKey::F1,
        Code::F2 => KbKey::F2,
        Code::F3 => KbKey::F3,
        Code::F4 => KbKey::F4,
        Code::F5 => KbKey::F5,
        Code::F6 => KbKey::F6,
        Code::F7 => KbKey::F7,
        Code::F8 => KbKey::F8,
        Code::F9 => KbKey::F9,
        Code::F10 => KbKey::F10,
        Code::F11 => KbKey::F11,
        Code::F12 => KbKey::F12,
        Code::Pause => KbKey::Pause,
        Code::ScrollLock => KbKey::ScrollLock,
        Code::PrintScreen => KbKey::PrintScreen,
        Code::Insert => KbKey::Insert,
        Code::Delete => KbKey::Delete,
        Code::Tab => KbKey::Tab,
        Code::Backspace => KbKey::Backspace,
        Code::ContextMenu => KbKey::ContextMenu,
        // kVK_JIS_Kana
        Code::Lang1 => KbKey::KanjiMode,
        // kVK_JIS_Eisu
        Code::Lang2 => KbKey::Eisu,
        Code::Home => KbKey::Home,
        Code::End => KbKey::End,
        Code::PageUp => KbKey::PageUp,
        Code::PageDown => KbKey::PageDown,
        Code::ArrowLeft => KbKey::ArrowLeft,
        Code::ArrowRight => KbKey::ArrowRight,
        Code::ArrowUp => KbKey::ArrowUp,
        Code::ArrowDown => KbKey::ArrowDown,
        Code::Enter => KbKey::Enter,
        Code::NumpadEnter => KbKey::Enter,
        Code::Help => KbKey::Help,
        _ => return None,
    })
}

fn is_valid_key(s: &str) -> bool {
    match s.chars().next() {
        None => false,
        Some(c) => c >= ' ' && c != '\x7f' && !('\u{e000}'..'\u{f900}').contains(&c),
    }
}

fn is_modifier_code(code: Code) -> bool {
    matches!(code, 
        Code::ShiftLeft
        | Code::ShiftRight
        | Code::AltLeft
        | Code::AltRight
        | Code::ControlLeft
        | Code::ControlRight
        | Code::MetaLeft
        | Code::MetaRight
        | Code::CapsLock
        | Code::Help)
}

impl KeyboardState {
    pub(crate) fn new() -> KeyboardState {
        let last_mods = NSEventModifierFlags::empty();
        KeyboardState { last_mods }
    }

    pub(crate) fn process_native_event(&mut self, event: id) -> Option<KeyEvent> {
        unsafe {
            let event_type = event.eventType();
            let key_code = event.keyCode();
            let code = key_code_to_code(key_code);
            let location = shared::code_to_location(code);
            let raw_mods = event.modifierFlags();
            let mods = make_modifiers(raw_mods);
            let state = match event_type {
                NSEventType::NSKeyDown => KeyState::Down,
                NSEventType::NSKeyUp => KeyState::Up,
                NSEventType::NSFlagsChanged => {
                    // We use `bits` here because we want to distinguish the
                    // device dependent bits (when both left and right keys
                    // may be pressed, for example).
                    let any_down = raw_mods.bits() & !self.last_mods.bits();
                    self.last_mods = raw_mods;
                    if is_modifier_code(code) {
                        if any_down == 0 {
                            KeyState::Up
                        } else {
                            KeyState::Down
                        }
                    } else {
                        // HandleFlagsChanged has some logic for this; it might
                        // happen when an app is deactivated by Command-Tab. In
                        // that case, the best thing to do is synthesize the event
                        // from the modifiers. But a challenge there is that we
                        // might get multiple events.
                        return None;
                    }
                }
                _ => unreachable!(),
            };
            let is_composing = false;
            let repeat: bool = event_type == NSEventType::NSKeyDown && msg_send![event, isARepeat];
            let key = if let Some(key) = code_to_key(code) {
                key
            } else {
                let characters = from_nsstring(event.characters());
                if is_valid_key(&characters) {
                    KbKey::Character(characters)
                } else {
                    let chars_ignoring = from_nsstring(event.charactersIgnoringModifiers());
                    if is_valid_key(&chars_ignoring) {
                        KbKey::Character(chars_ignoring)
                    } else {
                        // There may be more heroic things we can do here.
                        KbKey::Unidentified
                    }
                }
            };
            let event = KeyEvent {
                code,
                key,
                location,
                mods,
                state,
                is_composing,
                repeat,
            };
            Some(event)
        }
    }
}

const MODIFIER_MAP: &[(NSEventModifierFlags, Modifiers)] = &[
    (NSEventModifierFlags::NSShiftKeyMask, Modifiers::SHIFT),
    (NSEventModifierFlags::NSAlternateKeyMask, Modifiers::ALT),
    (NSEventModifierFlags::NSControlKeyMask, Modifiers::CONTROL),
    (NSEventModifierFlags::NSCommandKeyMask, Modifiers::META),
    (
        NSEventModifierFlags::NSAlphaShiftKeyMask,
        Modifiers::CAPS_LOCK,
    ),
];

pub(crate) fn make_modifiers(raw: NSEventModifierFlags) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    for &(flags, mods) in MODIFIER_MAP {
        if raw.contains(flags) {
            modifiers |= mods;
        }
    }
    modifiers
}
