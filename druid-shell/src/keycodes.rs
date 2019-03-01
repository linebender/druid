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
pub type Modifiers = u32;
// TODO: migrate this type to bitflags.

/// Modifier mask for alt key in `keydown` and `char` events.
pub const M_ALT: u32 = 1;

/// Modifier mask for control key in `keydown` and `char` events.
pub const M_CTRL: u32 = 2;

/// Modifier mask for shift key in `keydown` and `char` events.
pub const M_SHIFT: u32 = 4;

/// A specifier for a menu shortcut key.
pub struct MenuKey {
    /// Modifiers of menu key.
    /// 
    /// TODO: this should migrate to a bitflags type
    pub modifiers: u32,
    /// The unmodified key.
    pub key: KeySpec,
}

/// A specifier for a key (not including modifiers)
pub enum KeySpec {
    /// The key represents a single Unicode codepoint.
    Char(char),
    // TODO: other variants representing function keys, arrows, etc.
}

impl From<char> for MenuKey {
    fn from(c: char) -> MenuKey {
        MenuKey {
            modifiers: 0,
            key: c.into(),
        }
    }
}

impl From<char> for KeySpec {
    fn from(c: char) -> KeySpec {
        KeySpec::Char(c)
    }
}

pub fn command_modifier() -> u32;

impl MenuKey {
    pub fn command(k: impl Into<KeySpec>) -> MenuKey {
        MenuKey {
            modifiers: M_CTRL,
            key: k.into(),
        }
    }
}
