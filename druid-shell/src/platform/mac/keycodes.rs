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

//! macOS keycode handling.

use crate::keycodes::KeyCode;

pub type RawKeyCode = u16;

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
            //0x34 => unknown,
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
            //0x42 -> unknown,
            0x43 => KeyCode::NumpadMultiply,
            //0x44 => unknown,
            0x45 => KeyCode::NumpadAdd,
            //0x46 => unknown,
            0x47 => KeyCode::NumLock,
            //0x48 => KeypadClear,
            //0x49 => KeyCode::VolumeUp,
            //0x4a => KeyCode::VolumeDown,
            0x4b => KeyCode::NumpadDivide,
            0x4c => KeyCode::NumpadEnter,
            0x4e => KeyCode::NumpadSubtract,
            //0x4d => unknown,
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
            //0x5f => unknown,
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
            //0x6c => unknown,
            0x6d => KeyCode::F10,
            //0x6e => unknown,
            0x6f => KeyCode::F12,
            //0x70 => unknown,
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
            //0x7f =>  unknown,
            //0xa => KeyCode::Caret,
            other => KeyCode::Unknown(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vk_mac() {
        assert_eq!(KeyCode::from(0x30_u16), KeyCode::Tab);
        //F17
        assert_eq!(KeyCode::from(0x40_u16), KeyCode::Unknown(64));
    }
}
