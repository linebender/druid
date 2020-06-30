// Copyright 2020 The Druid Authors.
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

//! Keyboard types.

// This is a reasonable lint, but we keep signatures in sync with the
// bitflags implementation of the inner Modifiers type.
#![allow(clippy::trivially_copy_pass_by_ref)]

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

pub use keyboard_types::{Code, KeyState, Location};

/// The meaning (mapped value) of a keypress.
pub type KbKey = keyboard_types::Key;

/// Information about a keyboard event.
///
/// Note that this type is similar to [`KeyboardEvent`] in keyboard-types,
/// but has a few small differences for convenience. It is missing the `state`
/// field because that is already implicit in the event.
///
/// [`KeyboardEvent`]: keyboard_types::KeyboardEvent
#[non_exhaustive]
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct KeyEvent {
    /// Whether the key is pressed or released.
    pub state: KeyState,
    /// Logical key value.
    pub key: KbKey,
    /// Physical key position.
    pub code: Code,
    /// Location for keys with multiple instances on common keyboards.
    pub location: Location,
    /// Flags for pressed modifier keys.
    pub mods: Modifiers,
    /// True if the key is currently auto-repeated.
    pub repeat: bool,
    /// Events with this flag should be ignored in a text editor
    /// and instead composition events should be used.
    pub is_composing: bool,
}

/// The modifiers.
///
/// This type is a thin wrappers around [`keyboard_types::Modifiers`],
/// mostly for the convenience methods. If those get upstreamed, it
/// will simply become that type.
///
/// [`keyboard_types::Modifiers`]: keyboard_types::Modifiers
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Modifiers(keyboard_types::Modifiers);

/// A convenience trait for creating Key objects.
///
/// This trait is implemented by [`KbKey`] itself and also strings, which are
/// converted into the `Character` variant. It is defined this way and not
/// using the standard `Into` mechanism because `KbKey` is a type in an external
/// crate.
///
/// [`KbKey`]: KbKey
pub trait IntoKey {
    fn into_key(self) -> KbKey;
}

impl KeyEvent {
    #[doc(hidden)]
    /// Create a key event for testing purposes.
    pub fn for_test(mods: impl Into<Modifiers>, key: impl IntoKey) -> KeyEvent {
        let mods = mods.into();
        let key = key.into_key();
        KeyEvent {
            key,
            code: Code::Unidentified,
            location: Location::Standard,
            state: KeyState::Down,
            mods,
            is_composing: false,
            repeat: false,
        }
    }
}

impl Modifiers {
    pub const ALT: Modifiers = Modifiers(keyboard_types::Modifiers::ALT);
    pub const ALT_GRAPH: Modifiers = Modifiers(keyboard_types::Modifiers::ALT_GRAPH);
    pub const CAPS_LOCK: Modifiers = Modifiers(keyboard_types::Modifiers::CAPS_LOCK);
    pub const CONTROL: Modifiers = Modifiers(keyboard_types::Modifiers::CONTROL);
    pub const FN: Modifiers = Modifiers(keyboard_types::Modifiers::FN);
    pub const FN_LOCK: Modifiers = Modifiers(keyboard_types::Modifiers::FN_LOCK);
    pub const META: Modifiers = Modifiers(keyboard_types::Modifiers::META);
    pub const NUM_LOCK: Modifiers = Modifiers(keyboard_types::Modifiers::NUM_LOCK);
    pub const SCROLL_LOCK: Modifiers = Modifiers(keyboard_types::Modifiers::SCROLL_LOCK);
    pub const SHIFT: Modifiers = Modifiers(keyboard_types::Modifiers::SHIFT);
    pub const SYMBOL: Modifiers = Modifiers(keyboard_types::Modifiers::SYMBOL);
    pub const SYMBOL_LOCK: Modifiers = Modifiers(keyboard_types::Modifiers::SYMBOL_LOCK);
    pub const HYPER: Modifiers = Modifiers(keyboard_types::Modifiers::HYPER);
    pub const SUPER: Modifiers = Modifiers(keyboard_types::Modifiers::SUPER);

    /// Get the inner value.
    ///
    /// Note that this function might go away if our changes are upstreamed.
    pub fn raw(&self) -> keyboard_types::Modifiers {
        self.0
    }

    /// Determine whether Shift is set.
    pub fn shift(&self) -> bool {
        self.contains(Modifiers::SHIFT)
    }

    /// Determine whether Ctrl is set.
    pub fn ctrl(&self) -> bool {
        self.contains(Modifiers::CONTROL)
    }

    /// Determine whether Alt is set.
    pub fn alt(&self) -> bool {
        self.contains(Modifiers::ALT)
    }

    /// Determine whether Meta is set.
    pub fn meta(&self) -> bool {
        self.contains(Modifiers::META)
    }

    /// Returns an empty set of modifiers.
    pub fn empty() -> Modifiers {
        Default::default()
    }

    /// Returns `true` if no modifiers are set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if all the modifiers in `other` are set.
    pub fn contains(&self, other: Modifiers) -> bool {
        self.0.contains(other.0)
    }

    /// Inserts or removes the specified modifiers depending on the passed value.
    pub fn set(&mut self, other: Modifiers, value: bool) {
        self.0.set(other.0, value)
    }
}

impl BitAnd for Modifiers {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Modifiers(self.0 & rhs.0)
    }
}

impl BitAndAssign for Modifiers {
    // rhs is the "right-hand side" of the expression `a &= b`
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Modifiers(self.0 & rhs.0)
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Modifiers(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modifiers {
    // rhs is the "right-hand side" of the expression `a &= b`
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Modifiers(self.0 | rhs.0)
    }
}

impl BitXor for Modifiers {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        Modifiers(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Modifiers {
    // rhs is the "right-hand side" of the expression `a &= b`
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = Modifiers(self.0 ^ rhs.0)
    }
}

impl Not for Modifiers {
    type Output = Self;

    fn not(self) -> Self {
        Modifiers(!self.0)
    }
}

impl IntoKey for KbKey {
    fn into_key(self) -> KbKey {
        self
    }
}

impl IntoKey for &str {
    fn into_key(self) -> KbKey {
        KbKey::Character(self.into())
    }
}
