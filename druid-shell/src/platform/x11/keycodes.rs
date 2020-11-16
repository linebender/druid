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

//! X11 keycode handling.

use super::super::shared;
pub use super::super::shared::code_to_location;
use crate::keyboard::{Code, KbKey, Modifiers};
use x11rb::protocol::xproto::Keycode;

/// Convert a hardware scan code to a key.
///
/// Note: this is a hardcoded layout. We need to detect the user's
/// layout from the system and apply it.
pub fn code_to_key(code: Code, m: Modifiers) -> KbKey {
    fn a(s: &str) -> KbKey {
        KbKey::Character(s.into())
    }
    fn s(mods: Modifiers, base: &str, shifted: &str) -> KbKey {
        if mods.shift() {
            KbKey::Character(shifted.into())
        } else {
            KbKey::Character(base.into())
        }
    }
    fn n(mods: Modifiers, base: KbKey, num: &str) -> KbKey {
        if mods.contains(Modifiers::NUM_LOCK) != mods.shift() {
            KbKey::Character(num.into())
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

        Code::Escape => KbKey::Escape,
        Code::Backspace => KbKey::Backspace,
        Code::Tab => KbKey::Tab,
        Code::Enter => KbKey::Enter,
        Code::ControlLeft => KbKey::Control,
        Code::ShiftLeft => KbKey::Shift,
        Code::ShiftRight => KbKey::Shift,
        Code::NumpadMultiply => a("*"),
        Code::AltLeft => KbKey::Alt,
        Code::CapsLock => KbKey::CapsLock,
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
        Code::NumLock => KbKey::NumLock,
        Code::ScrollLock => KbKey::ScrollLock,
        Code::Numpad0 => n(m, KbKey::Insert, "0"),
        Code::Numpad1 => n(m, KbKey::End, "1"),
        Code::Numpad2 => n(m, KbKey::ArrowDown, "2"),
        Code::Numpad3 => n(m, KbKey::PageDown, "3"),
        Code::Numpad4 => n(m, KbKey::ArrowLeft, "4"),
        Code::Numpad5 => n(m, KbKey::Clear, "5"),
        Code::Numpad6 => n(m, KbKey::ArrowRight, "6"),
        Code::Numpad7 => n(m, KbKey::Home, "7"),
        Code::Numpad8 => n(m, KbKey::ArrowUp, "8"),
        Code::Numpad9 => n(m, KbKey::PageUp, "9"),
        Code::NumpadSubtract => a("-"),
        Code::NumpadAdd => a("+"),
        Code::NumpadDecimal => n(m, KbKey::Delete, "."),
        Code::IntlBackslash => s(m, "\\", "|"),
        Code::F11 => KbKey::F11,
        Code::F12 => KbKey::F12,
        // This mapping is based on the picture in the w3c spec.
        Code::IntlRo => a("\\"),
        Code::Convert => KbKey::Convert,
        Code::KanaMode => KbKey::KanaMode,
        Code::NonConvert => KbKey::NonConvert,
        Code::NumpadEnter => KbKey::Enter,
        Code::ControlRight => KbKey::Control,
        Code::NumpadDivide => a("/"),
        Code::PrintScreen => KbKey::PrintScreen,
        Code::AltRight => KbKey::Alt,
        Code::Home => KbKey::Home,
        Code::ArrowUp => KbKey::ArrowUp,
        Code::PageUp => KbKey::PageUp,
        Code::ArrowLeft => KbKey::ArrowLeft,
        Code::ArrowRight => KbKey::ArrowRight,
        Code::End => KbKey::End,
        Code::ArrowDown => KbKey::ArrowDown,
        Code::PageDown => KbKey::PageDown,
        Code::Insert => KbKey::Insert,
        Code::Delete => KbKey::Delete,
        Code::AudioVolumeMute => KbKey::AudioVolumeMute,
        Code::AudioVolumeDown => KbKey::AudioVolumeDown,
        Code::AudioVolumeUp => KbKey::AudioVolumeUp,
        Code::NumpadEqual => a("="),
        Code::Pause => KbKey::Pause,
        Code::NumpadComma => a(","),
        Code::Lang1 => KbKey::HangulMode,
        Code::Lang2 => KbKey::HanjaMode,
        Code::IntlYen => a("¥"),
        Code::MetaLeft => KbKey::Meta,
        Code::MetaRight => KbKey::Meta,
        Code::ContextMenu => KbKey::ContextMenu,
        Code::BrowserStop => KbKey::BrowserStop,
        Code::Again => KbKey::Again,
        Code::Props => KbKey::Props,
        Code::Undo => KbKey::Undo,
        Code::Select => KbKey::Select,
        Code::Copy => KbKey::Copy,
        Code::Open => KbKey::Open,
        Code::Paste => KbKey::Paste,
        Code::Find => KbKey::Find,
        Code::Cut => KbKey::Cut,
        Code::Help => KbKey::Help,
        Code::LaunchApp2 => KbKey::LaunchApplication2,
        Code::WakeUp => KbKey::WakeUp,
        Code::LaunchApp1 => KbKey::LaunchApplication1,
        Code::LaunchMail => KbKey::LaunchMail,
        Code::BrowserFavorites => KbKey::BrowserFavorites,
        Code::BrowserBack => KbKey::BrowserBack,
        Code::BrowserForward => KbKey::BrowserForward,
        Code::Eject => KbKey::Eject,
        Code::MediaTrackNext => KbKey::MediaTrackNext,
        Code::MediaPlayPause => KbKey::MediaPlayPause,
        Code::MediaTrackPrevious => KbKey::MediaTrackPrevious,
        Code::MediaStop => KbKey::MediaStop,
        Code::MediaSelect => KbKey::LaunchMediaPlayer,
        Code::BrowserHome => KbKey::BrowserHome,
        Code::BrowserRefresh => KbKey::BrowserRefresh,
        Code::BrowserSearch => KbKey::BrowserSearch,

        _ => KbKey::Unidentified,
    }
}

pub fn hardware_keycode_to_code(hw_keycode: Keycode) -> Code {
    shared::hardware_keycode_to_code(hw_keycode as u16)
}
