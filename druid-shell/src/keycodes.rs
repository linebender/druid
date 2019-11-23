// Copyright 2018 The xi-editor Authors.
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

//! Platform-independent key codes.

use crate::platform::keycodes as platform;

//NOTE: This was mostly taken from makepad, which I'm sure took it from somewhere else.
// I've written this out at least once before, for some xi-thing. The best resource
// I know of for this is probably the MDN keyboard event docs:
// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code

/// A platform-independent key identifier.
///
/// This ignores things like the user's keyboard layout.
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

    Unknown(platform::RawKeyCode),
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

/// This trait produces a mapping from KeyCodes, for physical and logical
/// keyboard function for keys such as those on the Numpad, where the notion of
/// modifier doesn't quite fit.
///
/// These functions are modeled loosely the `code` and `key`
/// fields KeyboardEvent Web API.
///
/// All platforms should implement this trait for KeyCode.
/// Callers can access this through druid_shell::UnknownKeyMap.
pub trait UnknownKeyMap {
    /// This is similar to the KeyboardEvent Web API code field.
    /// but produces a mapping from Unknown(platform::RawKeyCode), to physical KeyCodes.
    /// For example if passed an unknown keycode which correlates to
    /// Numpad0 with numlock off, it will return Numpad0.
    fn to_physical(self) -> Self;

    /// This is similar to the KeyboardEvent Web API key field.
    /// but produces a mapping from "Unknown keys", to logical KeyCode buttons.
    /// For example if passed an unknown keycode which correlates to
    /// Numpad0 with numlock off, it will return KeyCode::Insert.
    fn to_logical(self) -> Self;
}
