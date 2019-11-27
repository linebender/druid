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

/// For `KeyCode`s with duplicate variants of keys, For example `LeftShift` and
/// `RightShift` There exists single resolved key, `Shift` in this case.
/// `Special` exists in parallel to KeyCode and Takes Modifiers into account.
/// It does not however represent keys that result in textual characters.
///
/// A more complex example is `KeyCode::Numpad4` on platforms with `NumLock`
///
/// With the `NumLock` in the on state:
///     * `KeyCode::Numpad4` has a textual representation of the form "1",
///       So it does not have a `Special` representation.
/// When the `Numlock` is off:
///     * `KeyCode::Numpad4` might resolve to `Special::Left`.
///
/// In particular `Special` represents a logical mapping to the Platonic ideal
/// of key form. Providing the basis for a platform specific mapping.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Special {
    Add,
    Subtract,
    Multiply,
    Divide,

    Return,

    Alt,
    Control,
    Shift,
    Meta,

    Insert,
    Home,
    End,
    Delete,
    PageUp,
    PageDown,

    Down,
    Left,
    Right,
    Up,
}
