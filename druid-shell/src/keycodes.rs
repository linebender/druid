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

//! Keycode constants.

/// The type for keyboard modifiers.
// TODO: migrate this type to bitflags.
pub type Modifiers = u32;

/// Modifier mask for alt key in `keydown` and `char` events.
pub const M_ALT: u32 = 1;

/// Modifier mask for control key in `keydown` and `char` events.
pub const M_CTRL: u32 = 2;

/// Modifier mask for shift key in `keydown` and `char` events.
pub const M_SHIFT: u32 = 4;

/// Modifier mask for meta key in `keydown` and `char` events.
///
/// This is the windows key on Windows, the command key on macOS.
pub const M_META: u32 = 8;

/// A specifier for a menu shortcut key.
pub struct MenuKey {
    /// Modifiers of menu key.
    ///
    /// TODO: this should migrate to a bitflags type
    pub modifiers: u32,
    /// The unmodified key.
    pub key: KeySpec,
}

/// A specifier for a key (not including modifiers).
pub enum KeySpec {
    /// The key represents a single Unicode codepoint.
    Char(char),
    /// No key (ie a menu item with no accelerator).
    None,
    // TODO: other variants representing function keys, arrows, etc.
}

impl<K: Into<KeySpec>> From<K> for MenuKey {
    fn from(key: K) -> MenuKey {
        MenuKey {
            modifiers: 0,
            key: key.into(),
        }
    }
}

impl From<char> for KeySpec {
    fn from(c: char) -> KeySpec {
        KeySpec::Char(c)
    }
}

impl From<()> for KeySpec {
    fn from(_: ()) -> KeySpec {
        KeySpec::None
    }
}

/// Get the modifier corresponding to the default command key on the platform.
pub const fn command_modifier() -> Modifiers {
    #[cfg(target_os = "macos")]
    {
        M_META
    }
    #[cfg(not(target_os = "macos"))]
    {
        M_CTRL
    }
}

impl MenuKey {
    pub fn command(k: impl Into<KeySpec>) -> MenuKey {
        MenuKey {
            modifiers: command_modifier(),
            key: k.into(),
        }
    }

    /// The platform-standard menu key for the "quit application" action.
    pub const fn std_quit() -> MenuKey {
        #[cfg(target_os = "macos")]
        {
            MenuKey {
                modifiers: M_META,
                key: KeySpec::Char('q'),
            }
        }
        // TODO: Alt-F4 on Windows
        #[cfg(not(target_os = "macos"))]
        {
            MenuKey {
                modifiers: 0,
                key: KeySpec::None,
            }
        }
    }
}

// adapted from winapi and https://github.com/Eljay/directx
pub mod win_vks {
    pub const VK_LBUTTON: i32 = 0x01;
    pub const VK_RBUTTON: i32 = 0x02;
    pub const VK_CANCEL: i32 = 0x03;
    pub const VK_MBUTTON: i32 = 0x04;
    pub const VK_XBUTTON1: i32 = 0x05;
    pub const VK_XBUTTON2: i32 = 0x06;
    pub const VK_BACK: i32 = 0x08;
    pub const VK_TAB: i32 = 0x09;
    pub const VK_CLEAR: i32 = 0x0C;
    pub const VK_RETURN: i32 = 0x0D;
    pub const VK_SHIFT: i32 = 0x10;
    pub const VK_CONTROL: i32 = 0x11;
    pub const VK_MENU: i32 = 0x12;
    pub const VK_PAUSE: i32 = 0x13;
    pub const VK_CAPITAL: i32 = 0x14;
    pub const VK_KANA: i32 = 0x15;
    pub const VK_HANGEUL: i32 = 0x15;
    pub const VK_HANGUL: i32 = 0x15;
    pub const VK_JUNJA: i32 = 0x17;
    pub const VK_FINAL: i32 = 0x18;
    pub const VK_HANJA: i32 = 0x19;
    pub const VK_KANJI: i32 = 0x19;
    pub const VK_ESCAPE: i32 = 0x1B;
    pub const VK_CONVERT: i32 = 0x1C;
    pub const VK_NONCONVERT: i32 = 0x1D;
    pub const VK_ACCEPT: i32 = 0x1E;
    pub const VK_MODECHANGE: i32 = 0x1F;
    pub const VK_SPACE: i32 = 0x20;
    pub const VK_PRIOR: i32 = 0x21;
    pub const VK_NEXT: i32 = 0x22;
    pub const VK_END: i32 = 0x23;
    pub const VK_HOME: i32 = 0x24;
    pub const VK_LEFT: i32 = 0x25;
    pub const VK_UP: i32 = 0x26;
    pub const VK_RIGHT: i32 = 0x27;
    pub const VK_DOWN: i32 = 0x28;
    pub const VK_SELECT: i32 = 0x29;
    pub const VK_PRINT: i32 = 0x2A;
    pub const VK_EXECUTE: i32 = 0x2B;
    pub const VK_SNAPSHOT: i32 = 0x2C;
    pub const VK_INSERT: i32 = 0x2D;
    pub const VK_DELETE: i32 = 0x2E;
    pub const VK_HELP: i32 = 0x2F;
    pub const VK_LWIN: i32 = 0x5B;
    pub const VK_RWIN: i32 = 0x5C;
    pub const VK_APPS: i32 = 0x5D;
    pub const VK_SLEEP: i32 = 0x5F;
    pub const VK_NUMPAD0: i32 = 0x60;
    pub const VK_NUMPAD1: i32 = 0x61;
    pub const VK_NUMPAD2: i32 = 0x62;
    pub const VK_NUMPAD3: i32 = 0x63;
    pub const VK_NUMPAD4: i32 = 0x64;
    pub const VK_NUMPAD5: i32 = 0x65;
    pub const VK_NUMPAD6: i32 = 0x66;
    pub const VK_NUMPAD7: i32 = 0x67;
    pub const VK_NUMPAD8: i32 = 0x68;
    pub const VK_NUMPAD9: i32 = 0x69;
    pub const VK_MULTIPLY: i32 = 0x6A;
    pub const VK_ADD: i32 = 0x6B;
    pub const VK_SEPARATOR: i32 = 0x6C;
    pub const VK_SUBTRACT: i32 = 0x6D;
    pub const VK_DECIMAL: i32 = 0x6E;
    pub const VK_DIVIDE: i32 = 0x6F;
    pub const VK_F1: i32 = 0x70;
    pub const VK_F2: i32 = 0x71;
    pub const VK_F3: i32 = 0x72;
    pub const VK_F4: i32 = 0x73;
    pub const VK_F5: i32 = 0x74;
    pub const VK_F6: i32 = 0x75;
    pub const VK_F7: i32 = 0x76;
    pub const VK_F8: i32 = 0x77;
    pub const VK_F9: i32 = 0x78;
    pub const VK_F10: i32 = 0x79;
    pub const VK_F11: i32 = 0x7A;
    pub const VK_F12: i32 = 0x7B;
    pub const VK_F13: i32 = 0x7C;
    pub const VK_F14: i32 = 0x7D;
    pub const VK_F15: i32 = 0x7E;
    pub const VK_F16: i32 = 0x7F;
    pub const VK_F17: i32 = 0x80;
    pub const VK_F18: i32 = 0x81;
    pub const VK_F19: i32 = 0x82;
    pub const VK_F20: i32 = 0x83;
    pub const VK_F21: i32 = 0x84;
    pub const VK_F22: i32 = 0x85;
    pub const VK_F23: i32 = 0x86;
    pub const VK_F24: i32 = 0x87;
    pub const VK_NAVIGATION_VIEW: i32 = 0x88;
    pub const VK_NAVIGATION_MENU: i32 = 0x89;
    pub const VK_NAVIGATION_UP: i32 = 0x8A;
    pub const VK_NAVIGATION_DOWN: i32 = 0x8B;
    pub const VK_NAVIGATION_LEFT: i32 = 0x8C;
    pub const VK_NAVIGATION_RIGHT: i32 = 0x8D;
    pub const VK_NAVIGATION_ACCEPT: i32 = 0x8E;
    pub const VK_NAVIGATION_CANCEL: i32 = 0x8F;
    pub const VK_NUMLOCK: i32 = 0x90;
    pub const VK_SCROLL: i32 = 0x91;
    pub const VK_OEM_NEC_EQUAL: i32 = 0x92;
    pub const VK_OEM_FJ_JISHO: i32 = 0x92;
    pub const VK_OEM_FJ_MASSHOU: i32 = 0x93;
    pub const VK_OEM_FJ_TOUROKU: i32 = 0x94;
    pub const VK_OEM_FJ_LOYA: i32 = 0x95;
    pub const VK_OEM_FJ_ROYA: i32 = 0x96;
    pub const VK_LSHIFT: i32 = 0xA0;
    pub const VK_RSHIFT: i32 = 0xA1;
    pub const VK_LCONTROL: i32 = 0xA2;
    pub const VK_RCONTROL: i32 = 0xA3;
    pub const VK_LMENU: i32 = 0xA4;
    pub const VK_RMENU: i32 = 0xA5;
    pub const VK_BROWSER_BACK: i32 = 0xA6;
    pub const VK_BROWSER_FORWARD: i32 = 0xA7;
    pub const VK_BROWSER_REFRESH: i32 = 0xA8;
    pub const VK_BROWSER_STOP: i32 = 0xA9;
    pub const VK_BROWSER_SEARCH: i32 = 0xAA;
    pub const VK_BROWSER_FAVORITES: i32 = 0xAB;
    pub const VK_BROWSER_HOME: i32 = 0xAC;
    pub const VK_VOLUME_MUTE: i32 = 0xAD;
    pub const VK_VOLUME_DOWN: i32 = 0xAE;
    pub const VK_VOLUME_UP: i32 = 0xAF;
    pub const VK_MEDIA_NEXT_TRACK: i32 = 0xB0;
    pub const VK_MEDIA_PREV_TRACK: i32 = 0xB1;
    pub const VK_MEDIA_STOP: i32 = 0xB2;
    pub const VK_MEDIA_PLAY_PAUSE: i32 = 0xB3;
    pub const VK_LAUNCH_MAIL: i32 = 0xB4;
    pub const VK_LAUNCH_MEDIA_SELECT: i32 = 0xB5;
    pub const VK_LAUNCH_APP1: i32 = 0xB6;
    pub const VK_LAUNCH_APP2: i32 = 0xB7;
    pub const VK_OEM_1: i32 = 0xBA;
    pub const VK_OEM_PLUS: i32 = 0xBB;
    pub const VK_OEM_COMMA: i32 = 0xBC;
    pub const VK_OEM_MINUS: i32 = 0xBD;
    pub const VK_OEM_PERIOD: i32 = 0xBE;
    pub const VK_OEM_2: i32 = 0xBF;
    pub const VK_OEM_3: i32 = 0xC0;
    pub const VK_GAMEPAD_A: i32 = 0xC3;
    pub const VK_GAMEPAD_B: i32 = 0xC4;
    pub const VK_GAMEPAD_X: i32 = 0xC5;
    pub const VK_GAMEPAD_Y: i32 = 0xC6;
    pub const VK_GAMEPAD_RIGHT_SHOULDER: i32 = 0xC7;
    pub const VK_GAMEPAD_LEFT_SHOULDER: i32 = 0xC8;
    pub const VK_GAMEPAD_LEFT_TRIGGER: i32 = 0xC9;
    pub const VK_GAMEPAD_RIGHT_TRIGGER: i32 = 0xCA;
    pub const VK_GAMEPAD_DPAD_UP: i32 = 0xCB;
    pub const VK_GAMEPAD_DPAD_DOWN: i32 = 0xCC;
    pub const VK_GAMEPAD_DPAD_LEFT: i32 = 0xCD;
    pub const VK_GAMEPAD_DPAD_RIGHT: i32 = 0xCE;
    pub const VK_GAMEPAD_MENU: i32 = 0xCF;
    pub const VK_GAMEPAD_VIEW: i32 = 0xD0;
    pub const VK_GAMEPAD_LEFT_THUMBSTICK_BUTTON: i32 = 0xD1;
    pub const VK_GAMEPAD_RIGHT_THUMBSTICK_BUTTON: i32 = 0xD2;
    pub const VK_GAMEPAD_LEFT_THUMBSTICK_UP: i32 = 0xD3;
    pub const VK_GAMEPAD_LEFT_THUMBSTICK_DOWN: i32 = 0xD4;
    pub const VK_GAMEPAD_LEFT_THUMBSTICK_RIGHT: i32 = 0xD5;
    pub const VK_GAMEPAD_LEFT_THUMBSTICK_LEFT: i32 = 0xD6;
    pub const VK_GAMEPAD_RIGHT_THUMBSTICK_UP: i32 = 0xD7;
    pub const VK_GAMEPAD_RIGHT_THUMBSTICK_DOWN: i32 = 0xD8;
    pub const VK_GAMEPAD_RIGHT_THUMBSTICK_RIGHT: i32 = 0xD9;
    pub const VK_GAMEPAD_RIGHT_THUMBSTICK_LEFT: i32 = 0xDA;
    pub const VK_OEM_4: i32 = 0xDB;
    pub const VK_OEM_5: i32 = 0xDC;
    pub const VK_OEM_6: i32 = 0xDD;
    pub const VK_OEM_7: i32 = 0xDE;
    pub const VK_OEM_8: i32 = 0xDF;
    pub const VK_OEM_AX: i32 = 0xE1;
    pub const VK_OEM_102: i32 = 0xE2;
    pub const VK_ICO_HELP: i32 = 0xE3;
    pub const VK_ICO_00: i32 = 0xE4;
    pub const VK_PROCESSKEY: i32 = 0xE5;
    pub const VK_ICO_CLEAR: i32 = 0xE6;
    pub const VK_PACKET: i32 = 0xE7;
    pub const VK_OEM_RESET: i32 = 0xE9;
    pub const VK_OEM_JUMP: i32 = 0xEA;
    pub const VK_OEM_PA1: i32 = 0xEB;
    pub const VK_OEM_PA2: i32 = 0xEC;
    pub const VK_OEM_PA3: i32 = 0xED;
    pub const VK_OEM_WSCTRL: i32 = 0xEE;
    pub const VK_OEM_CUSEL: i32 = 0xEF;
    pub const VK_OEM_ATTN: i32 = 0xF0;
    pub const VK_OEM_FINISH: i32 = 0xF1;
    pub const VK_OEM_COPY: i32 = 0xF2;
    pub const VK_OEM_AUTO: i32 = 0xF3;
    pub const VK_OEM_ENLW: i32 = 0xF4;
    pub const VK_OEM_BACKTAB: i32 = 0xF5;
    pub const VK_ATTN: i32 = 0xF6;
    pub const VK_CRSEL: i32 = 0xF7;
    pub const VK_EXSEL: i32 = 0xF8;
    pub const VK_EREOF: i32 = 0xF9;
    pub const VK_PLAY: i32 = 0xFA;
    pub const VK_ZOOM: i32 = 0xFB;
    pub const VK_NONAME: i32 = 0xFC;
    pub const VK_PA1: i32 = 0xFD;
    pub const VK_OEM_CLEAR: i32 = 0xFE;
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
