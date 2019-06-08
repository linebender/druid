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
}
