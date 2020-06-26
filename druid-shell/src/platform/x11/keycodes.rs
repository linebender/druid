// Copyright 2020 The druid Authors.
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

//! X11 keycode handling.

use super::super::shared;
pub use super::super::shared::code_to_location;
use crate::keyboard_types::{Code, Key, Modifiers};
use x11rb::protocol::xproto::Keycode;

/// Convert a hardware scan code to a key.
///
/// Note: this is a hardcoded layout. We need to detect the user's
/// layout from the system and apply it.
pub fn code_to_key(code: Code, m: Modifiers) -> Key {
    fn a(s: &str) -> Key {
        Key::Character(s.into())
    }
    fn s(mods: Modifiers, base: &str, shifted: &str) -> Key {
        if mods.contains(Modifiers::SHIFT) {
            Key::Character(shifted.into())
        } else {
            Key::Character(base.into())
        }
    }
    fn n(mods: Modifiers, base: Key, num: &str) -> Key {
        if mods.contains(Modifiers::NUM_LOCK) != mods.contains(Modifiers::SHIFT) {
            Key::Character(num.into())
        } else {
            base
        }
    }
    match code {
        Code::KeyA => s(m, "a", "A"),
        Code::KeyB => s(m, "b", "B"),
        Code::KeyC => s(m, "c", "C"),
        Code::KeyD => s(m, "d", "D"),
        Code::KeyE => s(m, "e", "E"),
        Code::KeyF => s(m, "f", "F"),
        Code::KeyG => s(m, "g", "G"),
        Code::KeyH => s(m, "h", "H"),
        Code::KeyI => s(m, "i", "I"),
        Code::KeyJ => s(m, "j", "J"),
        Code::KeyK => s(m, "k", "K"),
        Code::KeyL => s(m, "l", "L"),
        Code::KeyM => s(m, "m", "M"),
        Code::KeyN => s(m, "n", "N"),
        Code::KeyO => s(m, "o", "O"),
        Code::KeyP => s(m, "p", "P"),
        Code::KeyQ => s(m, "q", "Q"),
        Code::KeyR => s(m, "r", "R"),
        Code::KeyS => s(m, "s", "S"),
        Code::KeyT => s(m, "t", "T"),
        Code::KeyU => s(m, "u", "U"),
        Code::KeyV => s(m, "v", "V"),
        Code::KeyW => s(m, "w", "W"),
        Code::KeyX => s(m, "x", "X"),
        Code::KeyY => s(m, "y", "Y"),
        Code::KeyZ => s(m, "z", "Z"),

        Code::Digit0 => s(m, "0", ")"),
        Code::Digit1 => s(m, "1", "!"),
        Code::Digit2 => s(m, "2", "@"),
        Code::Digit3 => s(m, "3", "#"),
        Code::Digit4 => s(m, "4", "$"),
        Code::Digit5 => s(m, "5", "%"),
        Code::Digit6 => s(m, "6", "^"),
        Code::Digit7 => s(m, "7", "&"),
        Code::Digit8 => s(m, "8", "*"),
        Code::Digit9 => s(m, "9", "("),

        Code::Backquote => s(m, "`", "~"),
        Code::Minus => s(m, "-", "_"),
        Code::Equal => s(m, "=", "+"),
        Code::BracketLeft => s(m, "[", "{"),
        Code::BracketRight => s(m, "]", "}"),
        Code::Backslash => s(m, "\\", "|"),
        Code::Semicolon => s(m, ";", ":"),
        Code::Quote => s(m, "'", "\""),
        Code::Comma => s(m, ",", "<"),
        Code::Period => s(m, ".", ">"),
        Code::Slash => s(m, "/", "?"),

        Code::Space => a(" "),

        Code::Escape => Key::Escape,
        Code::Backspace => Key::Backspace,
        Code::Tab => Key::Tab,
        Code::Enter => Key::Enter,
        Code::ControlLeft => Key::Control,
        Code::ShiftLeft => Key::Shift,
        Code::ShiftRight => Key::Shift,
        Code::NumpadMultiply => a("*"),
        Code::AltLeft => Key::Alt,
        Code::CapsLock => Key::CapsLock,
        Code::F1 => Key::F1,
        Code::F2 => Key::F2,
        Code::F3 => Key::F3,
        Code::F4 => Key::F4,
        Code::F5 => Key::F5,
        Code::F6 => Key::F6,
        Code::F7 => Key::F7,
        Code::F8 => Key::F8,
        Code::F9 => Key::F9,
        Code::F10 => Key::F10,
        Code::NumLock => Key::NumLock,
        Code::ScrollLock => Key::ScrollLock,
        Code::Numpad0 => n(m, Key::Insert, "0"),
        Code::Numpad1 => n(m, Key::End, "1"),
        Code::Numpad2 => n(m, Key::ArrowDown, "2"),
        Code::Numpad3 => n(m, Key::PageDown, "3"),
        Code::Numpad4 => n(m, Key::ArrowLeft, "4"),
        Code::Numpad5 => n(m, Key::Clear, "5"),
        Code::Numpad6 => n(m, Key::ArrowRight, "6"),
        Code::Numpad7 => n(m, Key::Home, "7"),
        Code::Numpad8 => n(m, Key::ArrowUp, "8"),
        Code::Numpad9 => n(m, Key::PageUp, "9"),
        Code::NumpadSubtract => a("-"),
        Code::NumpadAdd => a("+"),
        Code::NumpadDecimal => n(m, Key::Delete, "."),
        Code::IntlBackslash => s(m, "\\", "|"),
        Code::F11 => Key::F11,
        Code::F12 => Key::F12,
        // This mapping is based on the picture in the w3c spec.
        Code::IntlRo => a("\\"),
        Code::Convert => Key::Convert,
        Code::KanaMode => Key::KanaMode,
        Code::NonConvert => Key::NonConvert,
        Code::NumpadEnter => Key::Enter,
        Code::ControlRight => Key::Control,
        Code::NumpadDivide => a("/"),
        Code::PrintScreen => Key::PrintScreen,
        Code::AltRight => Key::Alt,
        Code::Home => Key::Home,
        Code::ArrowUp => Key::ArrowUp,
        Code::PageUp => Key::PageUp,
        Code::ArrowLeft => Key::ArrowLeft,
        Code::ArrowRight => Key::ArrowRight,
        Code::End => Key::End,
        Code::ArrowDown => Key::ArrowDown,
        Code::PageDown => Key::PageDown,
        Code::Insert => Key::Insert,
        Code::Delete => Key::Delete,
        Code::AudioVolumeMute => Key::AudioVolumeMute,
        Code::AudioVolumeDown => Key::AudioVolumeDown,
        Code::AudioVolumeUp => Key::AudioVolumeUp,
        Code::NumpadEqual => a("="),
        Code::Pause => Key::Pause,
        Code::NumpadComma => a(","),
        Code::Lang1 => Key::HangulMode,
        Code::Lang2 => Key::HanjaMode,
        Code::IntlYen => a("¥"),
        Code::MetaLeft => Key::Meta,
        Code::MetaRight => Key::Meta,
        Code::ContextMenu => Key::ContextMenu,
        Code::BrowserStop => Key::BrowserStop,
        Code::Again => Key::Again,
        Code::Props => Key::Props,
        Code::Undo => Key::Undo,
        Code::Select => Key::Select,
        Code::Copy => Key::Copy,
        Code::Open => Key::Open,
        Code::Paste => Key::Paste,
        Code::Find => Key::Find,
        Code::Cut => Key::Cut,
        Code::Help => Key::Help,
        Code::LaunchApp2 => Key::LaunchApplication2,
        Code::WakeUp => Key::WakeUp,
        Code::LaunchApp1 => Key::LaunchApplication1,
        Code::LaunchMail => Key::LaunchMail,
        Code::BrowserFavorites => Key::BrowserFavorites,
        Code::BrowserBack => Key::BrowserBack,
        Code::BrowserForward => Key::BrowserForward,
        Code::Eject => Key::Eject,
        Code::MediaTrackNext => Key::MediaTrackNext,
        Code::MediaPlayPause => Key::MediaPlayPause,
        Code::MediaTrackPrevious => Key::MediaTrackPrevious,
        Code::MediaStop => Key::MediaStop,
        Code::MediaSelect => Key::LaunchMediaPlayer,
        Code::BrowserHome => Key::BrowserHome,
        Code::BrowserRefresh => Key::BrowserRefresh,
        Code::BrowserSearch => Key::BrowserSearch,

        _ => Key::Unidentified,
    }
}

pub fn hardware_keycode_to_code(hw_keycode: Keycode) -> Code {
    shared::hardware_keycode_to_code(hw_keycode as u16)
}
