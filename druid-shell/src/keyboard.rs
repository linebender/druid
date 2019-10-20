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

use std::fmt;

/// A keyboard event, generated on every key press and key release.
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    /// The platform independent keycode.
    pub key_code: KeyCode,
    /// Whether or not this event is a repeat (the key was held down)
    pub is_repeat: bool,
    /// The modifiers for this event.
    pub mods: KeyModifiers,
    // these are exposed via methods, below. The rationale for this approach is
    // that a key might produce more than a single 'char' of input, but we don't
    // want to need a heap allocation in the trivial case. This gives us 15 bytes
    // of string storage, which... might be enough?
    text: TinyStr,
    /// The 'unmodified text' is the text that would be produced by this keystroke
    /// in the absence of ctrl+alt modifiers or preceding dead keys.
    unmodified_text: TinyStr,
    //TODO: add time
}

impl KeyEvent {
    /// Create a new `KeyEvent` struct. This accepts either &str or char for the last
    /// two arguments.
    pub(crate) fn new(
        key_code: impl Into<KeyCode>,
        is_repeat: bool,
        mods: KeyModifiers,
        text: impl Into<StrOrChar>,
        unmodified_text: impl Into<StrOrChar>,
    ) -> Self {
        let text = match text.into() {
            StrOrChar::Char(c) => c.into(),
            StrOrChar::Str(s) => TinyStr::new(s),
        };
        let unmodified_text = match unmodified_text.into() {
            StrOrChar::Char(c) => c.into(),
            StrOrChar::Str(s) => TinyStr::new(s),
        };

        KeyEvent {
            key_code: key_code.into(),
            is_repeat,
            mods,
            text,
            unmodified_text,
        }
    }

    /// The resolved input text for this event. This takes into account modifiers,
    /// e.g. the `chars` on macOS for opt+s is 'ß'.
    pub fn text(&self) -> Option<&str> {
        if self.text.len == 0 {
            None
        } else {
            Some(self.text.as_str())
        }
    }

    /// The unmodified input text for this event. On macOS, for opt+s, this is 's'.
    pub fn unmod_text(&self) -> Option<&str> {
        if self.unmodified_text.len == 0 {
            None
        } else {
            Some(self.unmodified_text.as_str())
        }
    }

    /// For creating `KeyEvent`s during testing.
    #[doc(hidden)]
    pub fn for_test(mods: impl Into<KeyModifiers>, text: &'static str, code: KeyCode) -> Self {
        KeyEvent::new(code, false, mods.into(), text, text)
    }
}

/// Keyboard modifier state, provided for events.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct KeyModifiers {
    /// Shift.
    pub shift: bool,
    /// Option on macOS.
    pub alt: bool,
    /// Control.
    pub ctrl: bool,
    /// Meta / Windows / Command
    pub meta: bool,
}

//NOTE: This was mostly taken from makepad, which I'm sure took it from somewhere else.
// I've written this out at least once before, for some xi-thing. The best resource
// I know of for this is probably the MDN keyboard event docs:
// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code
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
    LeftBracket,
    RightBracket,
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

    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    LeftShift,
    RightShift,
    /// command / windows / meta
    LeftMeta,
    RightMeta,

    Space,
    CapsLock,
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
    ScrollLock,
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
    NumLock,
    NumpadEnter,

    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    Unknown(RawKeyCode),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawKeyCode {
    Windows(i32),
    Mac(u16),
    Linux(u32),
}

impl KeyCode {
    pub fn is_printable(self) -> bool {
        use KeyCode::*;
        match self {
            Backtick | Key0 | Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7 | Key8 | Key9
            | Minus | Equals | Tab | KeyQ | KeyW | KeyE | KeyR | KeyT | KeyY | KeyU | KeyI
            | KeyO | KeyP | LeftBracket | RightBracket | Return | KeyA | KeyS | KeyD | KeyF
            | KeyG | KeyH | KeyJ | KeyK | KeyL | Semicolon | Quote | Backslash | KeyZ | KeyX
            | KeyC | KeyV | KeyB | KeyN | KeyM | Comma | Period | Slash | Space | Numpad0
            | Numpad1 | Numpad2 | Numpad3 | Numpad4 | Numpad5 | Numpad6 | Numpad7 | Numpad8
            | Numpad9 | NumpadEquals | NumpadSubtract | NumpadAdd | NumpadDecimal
            | NumpadMultiply | NumpadDivide | NumpadEnter => true,
            _ => false,
        }
    }
}

#[cfg(any(test, target_os = "macos"))]
impl From<u16> for KeyCode {
    fn from(raw: u16) -> KeyCode {
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
            0x1e => KeyCode::RightBracket,
            0x1f => KeyCode::KeyO,
            0x20 => KeyCode::KeyU,
            0x21 => KeyCode::LeftBracket,
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
            0x36 => KeyCode::RightMeta,
            0x37 => KeyCode::LeftMeta,
            0x38 => KeyCode::LeftShift,
            0x39 => KeyCode::CapsLock,
            0x3a => KeyCode::LeftAlt,
            0x3b => KeyCode::LeftControl,
            0x3c => KeyCode::RightShift,
            0x3d => KeyCode::RightAlt,
            0x3e => KeyCode::RightControl,
            //0x3f => Fn key,
            //0x40 => KeyCode::F17,
            0x41 => KeyCode::NumpadDecimal,
            //0x42 -> unkown,
            0x43 => KeyCode::NumpadMultiply,
            //0x44 => unkown,
            0x45 => KeyCode::NumpadAdd,
            //0x46 => unkown,
            0x47 => KeyCode::NumLock,
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
            other => KeyCode::Unknown(RawKeyCode::Mac(other.into())),
        }
    }
}

#[cfg(target_os = "windows")]
impl From<i32> for RawKeyCode {
    fn from(src: i32) -> RawKeyCode {
        RawKeyCode::Windows(src)
    }
}

#[cfg(target_os = "macos")]
impl From<u16> for RawKeyCode {
    fn from(src: u16) -> RawKeyCode {
        RawKeyCode::Mac(src)
    }
}

#[cfg(target_os = "windows")]
impl From<i32> for KeyCode {
    fn from(src: i32) -> KeyCode {
        use crate::keycodes::win_vks::*;
        use winapi::um::winuser::*;

        match src {
            VK_LSHIFT => KeyCode::LeftShift,
            VK_SHIFT => KeyCode::LeftShift,
            VK_RSHIFT => KeyCode::RightShift,
            VK_LCONTROL => KeyCode::LeftControl,
            VK_CONTROL => KeyCode::LeftControl,
            VK_RCONTROL => KeyCode::RightControl,
            VK_LMENU => KeyCode::LeftAlt,
            VK_MENU => KeyCode::LeftAlt,
            VK_RMENU => KeyCode::RightAlt,
            VK_OEM_PLUS => KeyCode::Equals,
            VK_OEM_COMMA => KeyCode::Comma,
            VK_OEM_MINUS => KeyCode::Minus,
            VK_OEM_PERIOD => KeyCode::Period,
            VK_OEM_1 => KeyCode::Semicolon,
            VK_OEM_2 => KeyCode::Slash,
            VK_OEM_3 => KeyCode::Backtick,
            VK_OEM_4 => KeyCode::LeftBracket,
            VK_OEM_5 => KeyCode::Backslash,
            VK_OEM_6 => KeyCode::RightBracket,
            VK_OEM_7 => KeyCode::Quote,
            VK_BACK => KeyCode::Backspace,
            VK_TAB => KeyCode::Tab,
            VK_RETURN => KeyCode::Return,
            VK_PAUSE => KeyCode::Pause,
            VK_ESCAPE => KeyCode::Escape,
            VK_SPACE => KeyCode::Space,
            VK_PRIOR => KeyCode::PageUp,
            VK_NEXT => KeyCode::PageDown,
            VK_END => KeyCode::End,
            VK_HOME => KeyCode::Home,
            VK_LEFT => KeyCode::ArrowLeft,
            VK_UP => KeyCode::ArrowUp,
            VK_RIGHT => KeyCode::ArrowRight,
            VK_DOWN => KeyCode::ArrowDown,
            VK_SNAPSHOT => KeyCode::PrintScreen,
            VK_INSERT => KeyCode::Insert,
            VK_DELETE => KeyCode::Delete,
            VK_CAPITAL => KeyCode::CapsLock,
            VK_NUMLOCK => KeyCode::NumLock,
            VK_SCROLL => KeyCode::ScrollLock,
            VK_0 => KeyCode::Key0,
            VK_1 => KeyCode::Key1,
            VK_2 => KeyCode::Key2,
            VK_3 => KeyCode::Key3,
            VK_4 => KeyCode::Key4,
            VK_5 => KeyCode::Key5,
            VK_6 => KeyCode::Key6,
            VK_7 => KeyCode::Key7,
            VK_8 => KeyCode::Key8,
            VK_9 => KeyCode::Key9,
            VK_A => KeyCode::KeyA,
            VK_B => KeyCode::KeyB,
            VK_C => KeyCode::KeyC,
            VK_D => KeyCode::KeyD,
            VK_E => KeyCode::KeyE,
            VK_F => KeyCode::KeyF,
            VK_G => KeyCode::KeyG,
            VK_H => KeyCode::KeyH,
            VK_I => KeyCode::KeyI,
            VK_J => KeyCode::KeyJ,
            VK_K => KeyCode::KeyK,
            VK_L => KeyCode::KeyL,
            VK_M => KeyCode::KeyM,
            VK_N => KeyCode::KeyN,
            VK_O => KeyCode::KeyO,
            VK_P => KeyCode::KeyP,
            VK_Q => KeyCode::KeyQ,
            VK_R => KeyCode::KeyR,
            VK_S => KeyCode::KeyS,
            VK_T => KeyCode::KeyT,
            VK_U => KeyCode::KeyU,
            VK_V => KeyCode::KeyV,
            VK_W => KeyCode::KeyW,
            VK_X => KeyCode::KeyX,
            VK_Y => KeyCode::KeyY,
            VK_Z => KeyCode::KeyZ,
            VK_NUMPAD0 => KeyCode::Numpad0,
            VK_NUMPAD1 => KeyCode::Numpad1,
            VK_NUMPAD2 => KeyCode::Numpad2,
            VK_NUMPAD3 => KeyCode::Numpad3,
            VK_NUMPAD4 => KeyCode::Numpad4,
            VK_NUMPAD5 => KeyCode::Numpad5,
            VK_NUMPAD6 => KeyCode::Numpad6,
            VK_NUMPAD7 => KeyCode::Numpad7,
            VK_NUMPAD8 => KeyCode::Numpad8,
            VK_NUMPAD9 => KeyCode::Numpad9,
            VK_MULTIPLY => KeyCode::NumpadMultiply,
            VK_ADD => KeyCode::NumpadAdd,
            VK_SUBTRACT => KeyCode::NumpadSubtract,
            VK_DECIMAL => KeyCode::NumpadDecimal,
            VK_DIVIDE => KeyCode::NumpadDivide,
            VK_F1 => KeyCode::F1,
            VK_F2 => KeyCode::F2,
            VK_F3 => KeyCode::F3,
            VK_F4 => KeyCode::F4,
            VK_F5 => KeyCode::F5,
            VK_F6 => KeyCode::F6,
            VK_F7 => KeyCode::F7,
            VK_F8 => KeyCode::F8,
            VK_F9 => KeyCode::F9,
            VK_F10 => KeyCode::F10,
            VK_F11 => KeyCode::F11,
            VK_F12 => KeyCode::F12,
            //VK_F13 => KeyCode::F13,
            //VK_F14 => KeyCode::F14,
            //VK_F15 => KeyCode::F15,
            //VK_F16 => KeyCode::F16,
            //VK_F17 => KeyCode::F17,
            //VK_F18 => KeyCode::F18,
            //VK_F19 => KeyCode::F19,
            //VK_F20 => KeyCode::F20,
            //VK_F21 => KeyCode::F21,
            //VK_F22 => KeyCode::F22,
            //VK_F23 => KeyCode::F23,
            //VK_F24 => KeyCode::F24,
            other => KeyCode::Unknown(other.into()),
        }
    }
}

#[cfg(any(target_os = "linux", feature = "use_gtk"))]
impl From<u32> for KeyCode {
    #[allow(non_upper_case_globals)]
    fn from(raw: u32) -> KeyCode {
        use gdk::enums::key::*;
        match raw {
            Escape => KeyCode::Escape,
            grave => KeyCode::Backtick,
            _0 => KeyCode::Key0,
            _1 => KeyCode::Key1,
            _2 => KeyCode::Key2,
            _3 => KeyCode::Key3,
            _4 => KeyCode::Key4,
            _5 => KeyCode::Key5,
            _6 => KeyCode::Key6,
            _7 => KeyCode::Key7,
            _8 => KeyCode::Key8,
            _9 => KeyCode::Key9,
            minus => KeyCode::Minus,
            equal => KeyCode::Equals,
            BackSpace => KeyCode::Backspace,

            Tab => KeyCode::Tab,
            q | Q => KeyCode::KeyQ,
            w | W => KeyCode::KeyW,
            e | E => KeyCode::KeyE,
            r | R => KeyCode::KeyR,
            t | T => KeyCode::KeyT,
            y | Y => KeyCode::KeyY,
            u | U => KeyCode::KeyU,
            i | I => KeyCode::KeyI,
            o | O => KeyCode::KeyO,
            p | P => KeyCode::KeyP,
            bracketleft => KeyCode::LeftBracket,
            bracketright => KeyCode::RightBracket,
            Return => KeyCode::Return,

            a | A => KeyCode::KeyA,
            s | S => KeyCode::KeyS,
            d | D => KeyCode::KeyD,
            f | F => KeyCode::KeyF,
            g | G => KeyCode::KeyG,
            h | H => KeyCode::KeyH,
            j | J => KeyCode::KeyJ,
            k | K => KeyCode::KeyK,
            l | L => KeyCode::KeyL,
            semicolon => KeyCode::Semicolon,
            quoteright => KeyCode::Quote,
            backslash => KeyCode::Backslash,

            z | Z => KeyCode::KeyZ,
            x | X => KeyCode::KeyX,
            c | C => KeyCode::KeyC,
            v | V => KeyCode::KeyV,
            b | B => KeyCode::KeyB,
            n | N => KeyCode::KeyN,
            m | M => KeyCode::KeyM,
            comma => KeyCode::Comma,
            period => KeyCode::Period,
            slash => KeyCode::Slash,

            Control_L => KeyCode::LeftControl,
            Control_R => KeyCode::RightControl,
            Alt_L => KeyCode::LeftAlt,
            Alt_R => KeyCode::RightAlt,
            Shift_L => KeyCode::LeftShift,
            Shift_R => KeyCode::RightShift,
            Meta_L => KeyCode::LeftMeta,
            Meta_R => KeyCode::RightMeta,

            space => KeyCode::Space,
            Caps_Lock => KeyCode::CapsLock,
            F1 => KeyCode::F1,
            F2 => KeyCode::F2,
            F3 => KeyCode::F3,
            F4 => KeyCode::F4,
            F5 => KeyCode::F5,
            F6 => KeyCode::F6,
            F7 => KeyCode::F7,
            F8 => KeyCode::F8,
            F9 => KeyCode::F9,
            F10 => KeyCode::F10,
            F11 => KeyCode::F11,
            F12 => KeyCode::F12,

            Print => KeyCode::PrintScreen,
            Scroll_Lock => KeyCode::ScrollLock,
            // Pause/Break not audio.
            Pause => KeyCode::Pause,

            Insert => KeyCode::Insert,
            Delete => KeyCode::Delete,
            Home => KeyCode::Home,
            End => KeyCode::End,
            Page_Up => KeyCode::PageUp,
            Page_Down => KeyCode::PageDown,

            KP_0 => KeyCode::Numpad0,
            KP_1 => KeyCode::Numpad1,
            KP_2 => KeyCode::Numpad2,
            KP_3 => KeyCode::Numpad3,
            KP_4 => KeyCode::Numpad4,
            KP_5 => KeyCode::Numpad5,
            KP_6 => KeyCode::Numpad6,
            KP_7 => KeyCode::Numpad7,
            KP_8 => KeyCode::Numpad8,
            KP_9 => KeyCode::Numpad9,

            KP_Equal => KeyCode::NumpadEquals,
            KP_Subtract => KeyCode::NumpadSubtract,
            KP_Add => KeyCode::NumpadAdd,
            KP_Decimal => KeyCode::NumpadDecimal,
            KP_Multiply => KeyCode::NumpadMultiply,
            KP_Divide => KeyCode::NumpadDivide,
            Num_Lock => KeyCode::NumLock,
            KP_Enter => KeyCode::NumpadEnter,

            Up => KeyCode::ArrowUp,
            Down => KeyCode::ArrowDown,
            Left => KeyCode::ArrowLeft,
            Right => KeyCode::ArrowRight,
            _ => {
                eprintln!("Warning: unknown keyval {}", raw);
                KeyCode::Unknown(RawKeyCode::Linux(raw))
            }
        }
    }
}

#[cfg(any(target_os = "linux", feature = "use_gtk"))]
impl Into<u32> for KeyCode {
    #[allow(non_upper_case_globals)]
    fn into(self) -> u32 {
        use gdk::enums::key::*;
        match self {
            KeyCode::Escape => Escape,
            KeyCode::Backtick => grave,
            KeyCode::Key0 => _0,
            KeyCode::Key1 => _1,
            KeyCode::Key2 => _2,
            KeyCode::Key3 => _3,
            KeyCode::Key4 => _4,
            KeyCode::Key5 => _5,
            KeyCode::Key6 => _6,
            KeyCode::Key7 => _7,
            KeyCode::Key8 => _8,
            KeyCode::Key9 => _9,
            KeyCode::Minus => minus,
            KeyCode::Equals => equal,
            KeyCode::Backspace => BackSpace,

            KeyCode::Tab => Tab,
            KeyCode::KeyQ => q | Q,
            KeyCode::KeyW => w | W,
            KeyCode::KeyE => e | E,
            KeyCode::KeyR => r | R,
            KeyCode::KeyT => t | T,
            KeyCode::KeyY => y | Y,
            KeyCode::KeyU => u | U,
            KeyCode::KeyI => i | I,
            KeyCode::KeyO => o | O,
            KeyCode::KeyP => p | P,
            KeyCode::LeftBracket => bracketleft,
            KeyCode::RightBracket => bracketright,
            KeyCode::Return => Return,

            KeyCode::KeyA => a | A,
            KeyCode::KeyS => s | S,
            KeyCode::KeyD => d | D,
            KeyCode::KeyF => f | F,
            KeyCode::KeyG => g | G,
            KeyCode::KeyH => h | H,
            KeyCode::KeyJ => j | J,
            KeyCode::KeyK => k | K,
            KeyCode::KeyL => l | L,
            KeyCode::Semicolon => semicolon,
            KeyCode::Quote => quoteright,
            KeyCode::Backslash => backslash,

            KeyCode::KeyZ => z | Z,
            KeyCode::KeyX => x | X,
            KeyCode::KeyC => c | C,
            KeyCode::KeyV => v | V,
            KeyCode::KeyB => b | B,
            KeyCode::KeyN => n | N,
            KeyCode::KeyM => m | M,
            KeyCode::Comma => comma,
            KeyCode::Period => period,
            KeyCode::Slash => slash,

            KeyCode::LeftControl => Control_L,
            KeyCode::RightControl => Control_R,
            KeyCode::LeftAlt => Alt_L,
            KeyCode::RightAlt => Alt_R,
            KeyCode::LeftShift => Shift_L,
            KeyCode::RightShift => Shift_R,
            KeyCode::LeftMeta => Meta_L,
            KeyCode::RightMeta => Meta_R,

            KeyCode::Space => space,
            KeyCode::CapsLock => Caps_Lock,
            KeyCode::F1 => F1,
            KeyCode::F2 => F2,
            KeyCode::F3 => F3,
            KeyCode::F4 => F4,
            KeyCode::F5 => F5,
            KeyCode::F6 => F6,
            KeyCode::F7 => F7,
            KeyCode::F8 => F8,
            KeyCode::F9 => F9,
            KeyCode::F10 => F10,
            KeyCode::F11 => F11,
            KeyCode::F12 => F12,

            KeyCode::PrintScreen => Print,
            KeyCode::ScrollLock => Scroll_Lock,
            // Pause/Break not audio.
            KeyCode::Pause => Pause,

            KeyCode::Insert => Insert,
            KeyCode::Delete => Delete,
            KeyCode::Home => Home,
            KeyCode::End => End,
            KeyCode::PageUp => Page_Up,
            KeyCode::PageDown => Page_Down,

            KeyCode::Numpad0 => KP_0,
            KeyCode::Numpad1 => KP_1,
            KeyCode::Numpad2 => KP_2,
            KeyCode::Numpad3 => KP_3,
            KeyCode::Numpad4 => KP_4,
            KeyCode::Numpad5 => KP_5,
            KeyCode::Numpad6 => KP_6,
            KeyCode::Numpad7 => KP_7,
            KeyCode::Numpad8 => KP_8,
            KeyCode::Numpad9 => KP_9,

            KeyCode::NumpadEquals => KP_Equal,
            KeyCode::NumpadSubtract => KP_Subtract,
            KeyCode::NumpadAdd => KP_Add,
            KeyCode::NumpadDecimal => KP_Decimal,
            KeyCode::NumpadMultiply => KP_Multiply,
            KeyCode::NumpadDivide => KP_Divide,
            KeyCode::NumLock => Num_Lock,
            KeyCode::NumpadEnter => KP_Enter,

            KeyCode::ArrowUp => Up,
            KeyCode::ArrowDown => Down,
            KeyCode::ArrowLeft => Left,
            KeyCode::ArrowRight => Right,
            KeyCode::Unknown(_) => unreachable!(
                "Unreachable: converting unknown KeyCode {:?} to a keyval",
                self
            ),
        }
    }
}

/// Should realistically be (8 * N) - 1; we need one byte for the length.
const TINY_STR_CAPACITY: usize = 15;

/// A stack allocated string with a fixed capacity.
#[derive(Clone, Copy)]
struct TinyStr {
    len: u8,
    buf: [u8; TINY_STR_CAPACITY],
}

impl TinyStr {
    fn new<S: AsRef<str>>(s: S) -> Self {
        let s = s.as_ref();
        let len = match s.len() {
            l @ 0..=15 => l,
            more => {
                // If some user has weird unicode bound to a key, it's better to
                // mishandle that input then to crash the client application?
                debug_assert!(
                    false,
                    "Err 101, Invalid Assumptions: TinyStr::new \
                     called with {} (len {}).",
                    s, more
                );
                #[cfg(test)]
                {
                    // we still want to fail when testing a release build
                    assert!(
                        false,
                        "Err 101, Invalid Assumptions: TinyStr::new \
                         called with {} (len {}).",
                        s, more
                    );
                }

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
        TinyStr {
            len: len as u8,
            buf,
        }
    }

    fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

impl From<char> for TinyStr {
    fn from(src: char) -> TinyStr {
        let len = src.len_utf8();
        let mut buf = [0; 15];
        src.encode_utf8(&mut buf);

        TinyStr {
            len: len as u8,
            buf,
        }
    }
}

/// A type we use in the constructor of `KeyEvent`, specifically to avoid exposing
/// internals.
pub enum StrOrChar {
    Char(char),
    Str(&'static str),
}

impl From<&'static str> for StrOrChar {
    fn from(src: &'static str) -> Self {
        StrOrChar::Str(src)
    }
}

impl From<char> for StrOrChar {
    fn from(src: char) -> StrOrChar {
        StrOrChar::Char(src)
    }
}

impl From<Option<char>> for StrOrChar {
    fn from(src: Option<char>) -> StrOrChar {
        match src {
            Some(c) => StrOrChar::Char(c),
            None => StrOrChar::Str(""),
        }
    }
}

impl fmt::Display for TinyStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for TinyStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TinyStr(\"{}\")", self.as_str())
    }
}

impl std::default::Default for TinyStr {
    fn default() -> Self {
        TinyStr::new("")
    }
}

impl fmt::Debug for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mods(")?;
        let mut has_prev = false;
        if self.meta {
            write!(f, "meta")?;
            has_prev = true;
        }
        if self.ctrl {
            if has_prev {
                write!(f, "+")?;
            }
            write!(f, "ctrl")?;
            has_prev = true;
        }
        if self.alt {
            if has_prev {
                write!(f, "+")?;
            }
            write!(f, "alt")?;
            has_prev = true;
        }
        if self.shift {
            if has_prev {
                write!(f, "+")?;
            }
            write!(f, "shift")?;
            has_prev = true;
        }
        if !has_prev {
            write!(f, "None")?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_panic(expected = "Invalid Assumptions")]
    fn smallstr() {
        let smol = TinyStr::new("hello");
        assert_eq!(smol.as_str(), "hello");
        let smol = TinyStr::new("");
        assert_eq!(smol.as_str(), "");
        let s_16 = "😍🥰😘😗";
        assert_eq!(s_16.len(), 16);
        assert!(!s_16.is_char_boundary(15));
        let _too_big = TinyStr::new("😍🥰😘😗");
    }

    #[test]
    fn vk_mac() {
        assert_eq!(KeyCode::from(0x30_u16), KeyCode::Tab);
        //F17
        assert_eq!(
            KeyCode::from(0x40_u16),
            KeyCode::Unknown(RawKeyCode::Mac(64))
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn win_vk() {
        assert_eq!(KeyCode::from(0x4F_i32), KeyCode::KeyO);
        // VK_ZOOM
        assert_eq!(
            KeyCode::from(0xFB_i32),
            KeyCode::Unknown(RawKeyCode::Windows(251))
        );
    }
}
