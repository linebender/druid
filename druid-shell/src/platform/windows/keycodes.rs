// Copyright 2019 The Druid Authors.
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

//! Windows keycode conversion.

use win_vks::*;
use winapi::um::winuser::*;

use crate::keycodes::KeyCode;

pub type RawKeyCode = i32;

mod win_vks {
    //NOTE: Most keys on Windows have VK_ values defined in use winapi::um::winuser,
    // but ASCII digits and letters are the exception. Fill those in here
    // (following https://github.com/Eljay/directx) so all keys have constants,
    // even though these are not official names for virtual-key codes.

    pub const VK_0: i32 = 0x30;
    pub const VK_1: i32 = 0x31;
    pub const VK_2: i32 = 0x32;
    pub const VK_3: i32 = 0x33;
    pub const VK_4: i32 = 0x34;
    pub const VK_5: i32 = 0x35;
    pub const VK_6: i32 = 0x36;
    pub const VK_7: i32 = 0x37;
    pub const VK_8: i32 = 0x38;
    pub const VK_9: i32 = 0x39;
    pub const VK_A: i32 = 0x41;
    pub const VK_B: i32 = 0x42;
    pub const VK_C: i32 = 0x43;
    pub const VK_D: i32 = 0x44;
    pub const VK_E: i32 = 0x45;
    pub const VK_F: i32 = 0x46;
    pub const VK_G: i32 = 0x47;
    pub const VK_H: i32 = 0x48;
    pub const VK_I: i32 = 0x49;
    pub const VK_J: i32 = 0x4A;
    pub const VK_K: i32 = 0x4B;
    pub const VK_L: i32 = 0x4C;
    pub const VK_M: i32 = 0x4D;
    pub const VK_N: i32 = 0x4E;
    pub const VK_O: i32 = 0x4F;
    pub const VK_P: i32 = 0x50;
    pub const VK_Q: i32 = 0x51;
    pub const VK_R: i32 = 0x52;
    pub const VK_S: i32 = 0x53;
    pub const VK_T: i32 = 0x54;
    pub const VK_U: i32 = 0x55;
    pub const VK_V: i32 = 0x56;
    pub const VK_W: i32 = 0x57;
    pub const VK_X: i32 = 0x58;
    pub const VK_Y: i32 = 0x59;
    pub const VK_Z: i32 = 0x5A;
}

macro_rules! map_keys {
    ($( $id:ident => $code:path),*) => {
        impl KeyCode {
            pub(crate) fn to_i32(&self) -> Option<i32> {
                // Some keycode are same value as others
                #[allow(unreachable_patterns)]
                match self {
                    $(
                        $code => Some($id)
                    ),*,
                    _ => None,
                }
            }
        }

        impl From<i32> for KeyCode {
            fn from(src: i32) -> KeyCode {
                match src {
                    $(
                        $id => $code
                    ),*,
                    other => KeyCode::Unknown(other),
                }
            }
        }
    }
}

map_keys! {
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
    VK_F12 => KeyCode::F12
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win_vk() {
        assert_eq!(KeyCode::from(0x4F_i32), KeyCode::KeyO);
        // VK_ZOOM
        assert_eq!(KeyCode::from(0xFB_i32), KeyCode::Unknown(251));
    }
}
