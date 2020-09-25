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

//! Hotkeys and helpers for parsing keyboard shortcuts.

use std::borrow::Borrow;

use log::warn;

use crate::{IntoKey, KbKey, KeyEvent, Modifiers};

// TODO: fix docstring

/// A description of a keyboard shortcut.
///
/// This type is only intended to be used to describe shortcuts,
/// and recognize them when they arrive.
///
/// # Examples
///
/// [`SysMods`] matches the Command key on macOS and Ctrl elsewhere:
///
/// ```
/// use druid_shell::{HotKey, KbKey, KeyEvent, RawMods, SysMods};
///
/// let hotkey = HotKey::new(SysMods::Cmd, "a");
///
/// #[cfg(target_os = "macos")]
/// assert!(hotkey.matches(KeyEvent::for_test(RawMods::Meta, "a")));
///
/// #[cfg(target_os = "windows")]
/// assert!(hotkey.matches(KeyEvent::for_test(RawMods::Ctrl, "a")));
/// ```
///
/// `None` matches only the key without modifiers:
///
/// ```
/// use druid_shell::{HotKey, KbKey, KeyEvent, RawMods, SysMods};
///
/// let hotkey = HotKey::new(None, KbKey::ArrowLeft);
///
/// assert!(hotkey.matches(KeyEvent::for_test(RawMods::None, KbKey::ArrowLeft)));
/// assert!(!hotkey.matches(KeyEvent::for_test(RawMods::Ctrl, KbKey::ArrowLeft)));
/// ```
///
/// [`SysMods`]: enum.SysMods.html
#[derive(Debug, Clone)]
pub struct HotKey {
    pub(crate) mods: RawMods,
    pub(crate) key: KbKey,
}

impl HotKey {
    /// Create a new hotkey.
    ///
    /// The first argument describes the keyboard modifiers. This can be `None`,
    /// or an instance of either [`SysMods`], or [`RawMods`]. [`SysMods`] unify the
    /// 'Command' key on macOS with the 'Ctrl' key on other platforms.
    ///
    /// The second argument describes the non-modifier key. This can be either
    /// a `&str` or a [`KbKey`]; the former is merely a convenient
    /// shorthand for `KbKey::Character()`.
    ///
    /// # Examples
    /// ```
    /// use druid_shell::{HotKey, KbKey, RawMods, SysMods};
    ///
    /// let select_all = HotKey::new(SysMods::Cmd, "a");
    /// let esc = HotKey::new(None, KbKey::Escape);
    /// let macos_fullscreen = HotKey::new(RawMods::CtrlMeta, "f");
    /// ```
    ///
    /// [`Key`]: keyboard_types::Key
    /// [`SysMods`]: enum.SysMods.html
    /// [`RawMods`]: enum.RawMods.html
    pub fn new(mods: impl Into<Option<RawMods>>, key: impl IntoKey) -> Self {
        HotKey {
            mods: mods.into().unwrap_or(RawMods::None),
            key: key.into_key(),
        }
        .warn_if_needed()
    }

    //TODO: figure out if we need to be normalizing case or something?
    fn warn_if_needed(self) -> Self {
        if let KbKey::Character(s) = &self.key {
            let km: Modifiers = self.mods.into();
            if km.shift() && s.chars().any(|c| c.is_lowercase()) {
                warn!(
                    "warning: HotKey {:?} includes shift, but text is lowercase. \
                     Text is matched literally; this may cause problems.",
                    &self
                );
            }
        }
        self
    }

    /// Returns `true` if this [`KeyEvent`] matches this `HotKey`.
    ///
    /// [`KeyEvent`]: KeyEvent
    pub fn matches(&self, event: impl Borrow<KeyEvent>) -> bool {
        // Should be a const but const bit_or doesn't work here.
        let base_mods = Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::META;
        let event = event.borrow();
        self.mods == event.mods & base_mods && self.key == event.key
    }
}

/// A platform-agnostic representation of keyboard modifiers, for command handling.
///
/// This does one thing: it allows specifying hotkeys that use the Command key
/// on macOS, but use the Ctrl key on other platforms.
#[derive(Debug, Clone, Copy)]
pub enum SysMods {
    None,
    Shift,
    /// Command on macOS, and Ctrl on windows/linux
    Cmd,
    /// Command + Alt on macOS, Ctrl + Alt on windows/linux
    AltCmd,
    /// Command + Shift on macOS, Ctrl + Shift on windows/linux
    CmdShift,
    /// Command + Alt + Shift on macOS, Ctrl + Alt + Shift on windows/linux
    AltCmdShift,
}

//TODO: should something like this just _replace_ keymodifiers?
/// A representation of the active modifier keys.
///
/// This is intended to be clearer than `Modifiers`, when describing hotkeys.
#[derive(Debug, Clone, Copy)]
pub enum RawMods {
    None,
    Alt,
    Ctrl,
    Meta,
    Shift,
    AltCtrl,
    AltMeta,
    AltShift,
    CtrlShift,
    CtrlMeta,
    MetaShift,
    AltCtrlMeta,
    AltCtrlShift,
    AltMetaShift,
    CtrlMetaShift,
    AltCtrlMetaShift,
}

impl std::cmp::PartialEq<Modifiers> for RawMods {
    fn eq(&self, other: &Modifiers) -> bool {
        let mods: Modifiers = (*self).into();
        mods == *other
    }
}

impl std::cmp::PartialEq<RawMods> for Modifiers {
    fn eq(&self, other: &RawMods) -> bool {
        other == self
    }
}

impl std::cmp::PartialEq<Modifiers> for SysMods {
    fn eq(&self, other: &Modifiers) -> bool {
        let mods: RawMods = (*self).into();
        mods == *other
    }
}

impl std::cmp::PartialEq<SysMods> for Modifiers {
    fn eq(&self, other: &SysMods) -> bool {
        let other: RawMods = (*other).into();
        &other == self
    }
}

impl From<RawMods> for Modifiers {
    fn from(src: RawMods) -> Modifiers {
        let (alt, ctrl, meta, shift) = match src {
            RawMods::None => (false, false, false, false),
            RawMods::Alt => (true, false, false, false),
            RawMods::Ctrl => (false, true, false, false),
            RawMods::Meta => (false, false, true, false),
            RawMods::Shift => (false, false, false, true),
            RawMods::AltCtrl => (true, true, false, false),
            RawMods::AltMeta => (true, false, true, false),
            RawMods::AltShift => (true, false, false, true),
            RawMods::CtrlMeta => (false, true, true, false),
            RawMods::CtrlShift => (false, true, false, true),
            RawMods::MetaShift => (false, false, true, true),
            RawMods::AltCtrlMeta => (true, true, true, false),
            RawMods::AltMetaShift => (true, false, true, true),
            RawMods::AltCtrlShift => (true, true, false, true),
            RawMods::CtrlMetaShift => (false, true, true, true),
            RawMods::AltCtrlMetaShift => (true, true, true, true),
        };
        let mut mods = Modifiers::empty();
        mods.set(Modifiers::ALT, alt);
        mods.set(Modifiers::CONTROL, ctrl);
        mods.set(Modifiers::META, meta);
        mods.set(Modifiers::SHIFT, shift);
        mods
    }
}

// we do this so that HotKey::new can accept `None` as an initial argument.
impl From<SysMods> for Option<RawMods> {
    fn from(src: SysMods) -> Option<RawMods> {
        Some(src.into())
    }
}

impl From<SysMods> for RawMods {
    fn from(src: SysMods) -> RawMods {
        #[cfg(target_os = "macos")]
        match src {
            SysMods::None => RawMods::None,
            SysMods::Shift => RawMods::Shift,
            SysMods::Cmd => RawMods::Meta,
            SysMods::AltCmd => RawMods::AltMeta,
            SysMods::CmdShift => RawMods::MetaShift,
            SysMods::AltCmdShift => RawMods::AltMetaShift,
        }
        #[cfg(not(target_os = "macos"))]
        match src {
            SysMods::None => RawMods::None,
            SysMods::Shift => RawMods::Shift,
            SysMods::Cmd => RawMods::Ctrl,
            SysMods::AltCmd => RawMods::AltCtrl,
            SysMods::CmdShift => RawMods::CtrlShift,
            SysMods::AltCmdShift => RawMods::AltCtrlShift,
        }
    }
}
