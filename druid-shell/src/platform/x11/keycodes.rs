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

use x11rb::protocol::xproto::Keycode;
use crate::keyboard_types::{Code, Key, Location, Modifiers};

// TODO: there's duplication here with gtk, as hardware keycodes
// are shared. There should be one source of truth.

/// Map hardware keycode to code.
///
/// In theory, the hardware keycode is device dependent, but in
/// practice it's probably pretty reliable.
///
/// The logic is based on NativeKeyToDOMCodeName.h
pub fn hardware_keycode_to_code(hw_keycode: Keycode) -> Code {
    match hw_keycode {
        0x0009 => Code::Escape,
        0x000A => Code::Digit1,
        0x000B => Code::Digit2,
        0x000C => Code::Digit3,
        0x000D => Code::Digit4,
        0x000E => Code::Digit5,
        0x000F => Code::Digit6,
        0x0010 => Code::Digit7,
        0x0011 => Code::Digit8,
        0x0012 => Code::Digit9,
        0x0013 => Code::Digit0,
        0x0014 => Code::Minus,
        0x0015 => Code::Equal,
        0x0016 => Code::Backspace,
        0x0017 => Code::Tab,
        0x0018 => Code::KeyQ,
        0x0019 => Code::KeyW,
        0x001A => Code::KeyE,
        0x001B => Code::KeyR,
        0x001C => Code::KeyT,
        0x001D => Code::KeyY,
        0x001E => Code::KeyU,
        0x001F => Code::KeyI,
        0x0020 => Code::KeyO,
        0x0021 => Code::KeyP,
        0x0022 => Code::BracketLeft,
        0x0023 => Code::BracketRight,
        0x0024 => Code::Enter,
        0x0025 => Code::ControlLeft,
        0x0026 => Code::KeyA,
        0x0027 => Code::KeyS,
        0x0028 => Code::KeyD,
        0x0029 => Code::KeyF,
        0x002A => Code::KeyG,
        0x002B => Code::KeyH,
        0x002C => Code::KeyJ,
        0x002D => Code::KeyK,
        0x002E => Code::KeyL,
        0x002F => Code::Semicolon,
        0x0030 => Code::Quote,
        0x0031 => Code::Backquote,
        0x0032 => Code::ShiftLeft,
        0x0033 => Code::Backslash,
        0x0034 => Code::KeyZ,
        0x0035 => Code::KeyX,
        0x0036 => Code::KeyC,
        0x0037 => Code::KeyV,
        0x0038 => Code::KeyB,
        0x0039 => Code::KeyN,
        0x003A => Code::KeyM,
        0x003B => Code::Comma,
        0x003C => Code::Period,
        0x003D => Code::Slash,
        0x003E => Code::ShiftRight,
        0x003F => Code::NumpadMultiply,
        0x0040 => Code::AltLeft,
        0x0041 => Code::Space,
        0x0042 => Code::CapsLock,
        0x0043 => Code::F1,
        0x0044 => Code::F2,
        0x0045 => Code::F3,
        0x0046 => Code::F4,
        0x0047 => Code::F5,
        0x0048 => Code::F6,
        0x0049 => Code::F7,
        0x004A => Code::F8,
        0x004B => Code::F9,
        0x004C => Code::F10,
        0x004D => Code::NumLock,
        0x004E => Code::ScrollLock,
        0x004F => Code::Numpad7,
        0x0050 => Code::Numpad8,
        0x0051 => Code::Numpad9,
        0x0052 => Code::NumpadSubtract,
        0x0053 => Code::Numpad4,
        0x0054 => Code::Numpad5,
        0x0055 => Code::Numpad6,
        0x0056 => Code::NumpadAdd,
        0x0057 => Code::Numpad1,
        0x0058 => Code::Numpad2,
        0x0059 => Code::Numpad3,
        0x005A => Code::Numpad0,
        0x005B => Code::NumpadDecimal,
        0x005E => Code::IntlBackslash,
        0x005F => Code::F11,
        0x0060 => Code::F12,
        0x0061 => Code::IntlRo,
        0x0064 => Code::Convert,
        0x0065 => Code::KanaMode,
        0x0066 => Code::NonConvert,
        0x0068 => Code::NumpadEnter,
        0x0069 => Code::ControlRight,
        0x006A => Code::NumpadDivide,
        0x006B => Code::PrintScreen,
        0x006C => Code::AltRight,
        0x006E => Code::Home,
        0x006F => Code::ArrowUp,
        0x0070 => Code::PageUp,
        0x0071 => Code::ArrowLeft,
        0x0072 => Code::ArrowRight,
        0x0073 => Code::End,
        0x0074 => Code::ArrowDown,
        0x0075 => Code::PageDown,
        0x0076 => Code::Insert,
        0x0077 => Code::Delete,
        0x0079 => Code::AudioVolumeMute,
        0x007A => Code::AudioVolumeDown,
        0x007B => Code::AudioVolumeUp,
        0x007D => Code::NumpadEqual,
        0x007F => Code::Pause,
        0x0081 => Code::NumpadComma,
        0x0082 => Code::Lang1,
        0x0083 => Code::Lang2,
        0x0084 => Code::IntlYen,
        0x0085 => Code::MetaLeft,
        0x0086 => Code::MetaRight,
        0x0087 => Code::ContextMenu,
        0x0088 => Code::BrowserStop,
        0x0089 => Code::Again,
        0x008A => Code::Props,
        0x008B => Code::Undo,
        0x008C => Code::Select,
        0x008D => Code::Copy,
        0x008E => Code::Open,
        0x008F => Code::Paste,
        0x0090 => Code::Find,
        0x0091 => Code::Cut,
        0x0092 => Code::Help,
        0x0094 => Code::LaunchApp2,
        0x0097 => Code::WakeUp,
        0x0098 => Code::LaunchApp1,
        // key to right of volume controls on T430s produces 0x9C
        // but no documentation of what it should map to :/
        0x00A3 => Code::LaunchMail,
        0x00A4 => Code::BrowserFavorites,
        0x00A6 => Code::BrowserBack,
        0x00A7 => Code::BrowserForward,
        0x00A9 => Code::Eject,
        0x00AB => Code::MediaTrackNext,
        0x00AC => Code::MediaPlayPause,
        0x00AD => Code::MediaTrackPrevious,
        0x00AE => Code::MediaStop,
        0x00B3 => Code::MediaSelect,
        0x00B4 => Code::BrowserHome,
        0x00B5 => Code::BrowserRefresh,
        0x00E1 => Code::BrowserSearch,
        _ => Code::Unidentified,
    }
}

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

/// Map key code to location.
///
/// This is cut and pasted from the mac version. There should probably
/// be a single source of truth.
pub fn code_to_location(code: Code) -> Location {
    match code {
        Code::MetaLeft | Code::ShiftLeft | Code::AltLeft | Code::ControlLeft => Location::Left,
        Code::MetaRight | Code::ShiftRight | Code::AltRight | Code::ControlRight => Location::Right,
        Code::Numpad0
        | Code::Numpad1
        | Code::Numpad2
        | Code::Numpad3
        | Code::Numpad4
        | Code::Numpad5
        | Code::Numpad6
        | Code::Numpad7
        | Code::Numpad8
        | Code::Numpad9
        | Code::NumpadAdd
        | Code::NumpadComma
        | Code::NumpadDecimal
        | Code::NumpadDivide
        | Code::NumpadEnter
        | Code::NumpadEqual
        | Code::NumpadMultiply
        | Code::NumpadSubtract => Location::Numpad,
        _ => Location::Standard,
    }
}
