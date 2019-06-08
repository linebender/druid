// Copyright 2019 The xi-editor Authors.
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

//! Keyboard event types and helpers
//!
/// A key event.
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    /// The platform independent keycode.
    pub key_code: KeyCode,
    /// Whether or not this event is a repeat (the key was held down)
    pub is_repeat: bool,
    /// The modifiers for this event.
    pub modifiers: KeyModifiers,
    // these are exposed via methods, below. The rationale for this approach is
    // that a key might produce more than a single 'char' of input, but we don't
    // want to need a heap allocation in the trivial case. This gives us 15 bytes
    // of string storage, which... might be enough?
    pub(crate) chars: SmallStr,
    pub(crate) unmodified_chars: SmallStr,
    //TODO: add time
}

impl KeyEvent {
    /// The resolved input text for this event. This takes into account modifiers,
    /// e.g. the `chars` on macOS for opt+s is 'ß'.
    pub fn chars(&self) -> Option<&str> {
        if self.chars.len == 0 {
            None
        } else {
            Some(self.chars.as_str())
        }
    }

    /// The unmodified input text for this event. On macOS, for opt+s, this is 's'.
    pub fn unmod_chars(&self) -> Option<&str> {
        if self.unmodified_chars.len == 0 {
            None
        } else {
            Some(self.unmodified_chars.as_str())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KeyModifiers {
    pub shift: bool,
    /// Option on macOS.
    pub alt: bool,
    /// Control.
    pub ctrl: bool,
    /// Meta / Windows / Command
    pub meta: bool,
}

/// A platform-independent key identifier. This ignores things like the user's
/// keyboard layout.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyCode {
    Escape,

    Backtick,
    /// The numeral 0 in the top row.
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Minus,
    Equals,

    Backspace,
    Tab,

    KeyQ,
    KeyW,
    KeyE,
    KeyR,
    KeyT,
    KeyY,
    KeyU,
    KeyI,
    KeyO,
    KeyP,
    /// '['
    LBracket,
    RBracket,
    Return,

    KeyA,
    KeyS,
    KeyD,
    KeyF,
    KeyG,
    KeyH,
    KeyJ,
    KeyK,
    KeyL,
    Semicolon,
    Quote,
    Backslash,

    KeyZ,
    KeyX,
    KeyC,
    KeyV,
    KeyB,
    KeyN,
    KeyM,
    Comma,
    Period,
    Slash,

    Control,
    Alt,
    Shift,
    /// command / windows / meta
    Meta,
    Menu,

    Space,
    Capslock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    PrintScreen,
    Scrolllock,
    Pause,

    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,

    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    NumpadEquals,
    NumpadSubtract,
    NumpadAdd,
    NumpadDecimal,
    NumpadMultiply,
    NumpadDivide,
    Numlock,
    NumpadEnter,

    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    Unknown,
}

impl KeyCode {
    #[cfg(target_os = "macos")]
    pub(crate) fn from_mac_vk_code(raw: u16) -> Self {
        match raw {
            0x00 => KeyCode::KeyA,
            0x01 => KeyCode::KeyS,
            0x02 => KeyCode::KeyD,
            0x03 => KeyCode::KeyF,
            0x04 => KeyCode::KeyH,
            0x05 => KeyCode::KeyG,
            0x06 => KeyCode::KeyZ,
            0x07 => KeyCode::KeyX,
            0x08 => KeyCode::KeyC,
            0x09 => KeyCode::KeyV,
            //0x0a => World 1,
            0x0b => KeyCode::KeyB,
            0x0c => KeyCode::KeyQ,
            0x0d => KeyCode::KeyW,
            0x0e => KeyCode::KeyE,
            0x0f => KeyCode::KeyR,
            0x10 => KeyCode::KeyY,
            0x11 => KeyCode::KeyT,
            0x12 => KeyCode::Key1,
            0x13 => KeyCode::Key2,
            0x14 => KeyCode::Key3,
            0x15 => KeyCode::Key4,
            0x16 => KeyCode::Key6,
            0x17 => KeyCode::Key5,
            0x18 => KeyCode::Equals,
            0x19 => KeyCode::Key9,
            0x1a => KeyCode::Key7,
            0x1b => KeyCode::Minus,
            0x1c => KeyCode::Key8,
            0x1d => KeyCode::Key0,
            0x1e => KeyCode::RBracket,
            0x1f => KeyCode::KeyO,
            0x20 => KeyCode::KeyU,
            0x21 => KeyCode::LBracket,
            0x22 => KeyCode::KeyI,
            0x23 => KeyCode::KeyP,
            0x24 => KeyCode::Return,
            0x25 => KeyCode::KeyL,
            0x26 => KeyCode::KeyJ,
            0x27 => KeyCode::Backtick,
            0x28 => KeyCode::KeyK,
            0x29 => KeyCode::Semicolon,
            0x2a => KeyCode::Backslash,
            0x2b => KeyCode::Comma,
            0x2c => KeyCode::Slash,
            0x2d => KeyCode::KeyN,
            0x2e => KeyCode::KeyM,
            0x2f => KeyCode::Period,
            0x30 => KeyCode::Tab,
            0x31 => KeyCode::Space,
            0x32 => KeyCode::Backtick,
            0x33 => KeyCode::Backspace,
            //0x34 => unkown,
            0x35 => KeyCode::Escape,
            //0x36 => KeyCode::RLogo,
            //0x37 => KeyCode::LLogo,
            //0x38 => KeyCode::LShift,
            0x39 => KeyCode::Capslock,
            //0x3a => KeyCode::LAlt,
            //0x3b => KeyCode::LControl,
            //0x3c => KeyCode::RShift,
            //0x3d => KeyCode::RAlt,
            //0x3e => KeyCode::RControl,
            //0x3f => Fn key,
            //0x40 => KeyCode::F17,
            0x41 => KeyCode::NumpadDecimal,
            //0x42 -> unkown,
            0x43 => KeyCode::NumpadMultiply,
            //0x44 => unkown,
            0x45 => KeyCode::NumpadAdd,
            //0x46 => unkown,
            0x47 => KeyCode::Numlock,
            //0x48 => KeypadClear,
            //0x49 => KeyCode::VolumeUp,
            //0x4a => KeyCode::VolumeDown,
            0x4b => KeyCode::NumpadDivide,
            0x4c => KeyCode::NumpadEnter,
            0x4e => KeyCode::NumpadSubtract,
            //0x4d => unkown,
            //0x4e => KeyCode::Subtract,
            //0x4f => KeyCode::F18,
            //0x50 => KeyCode::F19,
            0x51 => KeyCode::NumpadEquals,
            0x52 => KeyCode::Numpad0,
            0x53 => KeyCode::Numpad1,
            0x54 => KeyCode::Numpad2,
            0x55 => KeyCode::Numpad3,
            0x56 => KeyCode::Numpad4,
            0x57 => KeyCode::Numpad5,
            0x58 => KeyCode::Numpad6,
            0x59 => KeyCode::Numpad7,
            //0x5a => KeyCode::F20,
            0x5b => KeyCode::Numpad8,
            0x5c => KeyCode::Numpad9,
            //0x5d => KeyCode::Yen,
            //0x5e => JIS Ro,
            //0x5f => unkown,
            0x60 => KeyCode::F5,
            0x61 => KeyCode::F6,
            0x62 => KeyCode::F7,
            0x63 => KeyCode::F3,
            0x64 => KeyCode::F8,
            0x65 => KeyCode::F9,
            //0x66 => JIS Eisuu (macOS),
            0x67 => KeyCode::F11,
            //0x68 => JIS Kana (macOS),
            0x69 => KeyCode::PrintScreen,
            //0x6a => KeyCode::F16,
            //0x6b => KeyCode::F14,
            //0x6c => unkown,
            0x6d => KeyCode::F10,
            //0x6e => unkown,
            0x6f => KeyCode::F12,
            //0x70 => unkown,
            //0x71 => KeyCode::F15,
            0x72 => KeyCode::Insert,
            0x73 => KeyCode::Home,
            0x74 => KeyCode::PageUp,
            0x75 => KeyCode::Delete,
            0x76 => KeyCode::F4,
            0x77 => KeyCode::End,
            0x78 => KeyCode::F2,
            0x79 => KeyCode::PageDown,
            0x7a => KeyCode::F1,
            0x7b => KeyCode::ArrowLeft,
            0x7c => KeyCode::ArrowRight,
            0x7d => KeyCode::ArrowDown,
            0x7e => KeyCode::ArrowUp,
            //0x7f =>  unkown,
            //0xa => KeyCode::Caret,
            _ => KeyCode::Unknown,
        }
    }
}

/// A stack allocated string.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SmallStr {
    pub(crate) len: u8,
    pub(crate) buf: [u8; 15],
}

impl SmallStr {
    pub(crate) fn new<S: AsRef<str>>(s: S) -> Self {
        let s = s.as_ref();
        let len = match s.len() {
            l @ 0..=15 => l,
            more => {
                debug_assert!(
                    false,
                    "Err 101, Invalid Assumptions: SmallStr::new called with {} (len {}).",
                    s, more
                );
                // rups. find last valid codepoint offset
                let mut len = 15;
                while !s.is_char_boundary(len) {
                    len -= 1;
                }
                len
            }
        };
        let mut buf = [0; 15];
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        SmallStr {
            len: len as u8,
            buf,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_panic(expected("Invalid Assumptions"))]
    fn smallstr() {
        let smol = SmallStr::new("hello");
        assert_eq!(smol.as_str(), "hello");
        let smol = SmallStr::new("");
        assert_eq!(smol.as_str(), "");
        let s_16 = "😍🥰😘😗";
        assert_eq!(s_16.len(), 16);
        assert!(!s_16.is_char_boundary(15));
        let _too_big = SmallStr::new("😍🥰😘😗");
    }
}
