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
