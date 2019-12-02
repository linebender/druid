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

//! Keyboard event types and helpers

use super::keycodes::KeyCode;
use std::fmt;

/// A keyboard event, generated on every key press and key release.
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    /// The platform independent keycode.
    pub key_code: KeyCode,
    /// Whether or not this event is a repeat (the key was held down)
    pub is_repeat: bool,
    /// The modifiers for this event.
    pub mods: KeyModifiers,
    // these are exposed via methods, below. The rationale for this approach is
    // that a key might produce more than a single 'char' of input, but we don't
    // want to need a heap allocation in the trivial case. This gives us 15 bytes
    // of string storage, which... might be enough?
    text: TinyStr,
    /// The 'unmodified text' is the text that would be produced by this keystroke
    /// in the absence of ctrl+alt modifiers or preceding dead keys.
    unmodified_text: TinyStr,
    //TODO: add time
}

/// A modifier, for the purpose of matching key events.
pub enum Modifier {
    Alt,
    Ctrl,
    Meta,
    Shift,
    // The command key; maps to Meta on macOS, Ctrl otherwise.
    Cmd,
    NoMods,
}

pub trait KeyMatcher {
    fn matches(&self, key_event: &KeyEvent) -> bool {
        self.matches_internal(key_event, KeyModifiers::default())
    }

    #[doc(hidden)]
    #[allow(unused)]
    fn matches_internal(&self, key_event: &KeyEvent, covered: KeyModifiers) -> bool {
        false
    }
}

#[doc(hidden)]
pub trait ModMatcher {
    /// Determine whether the modifiers match the spec.
    ///
    /// On success, return the set of modifiers "covered" by the spec.
    fn match_mods(&self, mods: KeyModifiers) -> Option<KeyModifiers>;
}

impl KeyEvent {
    /// Create a new `KeyEvent` struct. This accepts either &str or char for the last
    /// two arguments.
    pub(crate) fn new(
        key_code: impl Into<KeyCode>,
        is_repeat: bool,
        mods: KeyModifiers,
        text: impl Into<StrOrChar>,
        unmodified_text: impl Into<StrOrChar>,
    ) -> Self {
        let text = match text.into() {
            StrOrChar::Char(c) => c.into(),
            StrOrChar::Str(s) => TinyStr::new(s),
        };
        let unmodified_text = match unmodified_text.into() {
            StrOrChar::Char(c) => c.into(),
            StrOrChar::Str(s) => TinyStr::new(s),
        };

        KeyEvent {
            key_code: key_code.into(),
            is_repeat,
            mods,
            text,
            unmodified_text,
        }
    }

    /// The resolved input text for this event. This takes into account modifiers,
    /// e.g. the `chars` on macOS for opt+s is 'ß'.
    pub fn text(&self) -> Option<&str> {
        if self.text.len == 0 {
            None
        } else {
            Some(self.text.as_str())
        }
    }

    /// The unmodified input text for this event. On macOS, for opt+s, this is 's'.
    pub fn unmod_text(&self) -> Option<&str> {
        if self.unmodified_text.len == 0 {
            None
        } else {
            Some(self.unmodified_text.as_str())
        }
    }

    /// Test whether the key matches the given pattern.
    ///
    /// # Examples
    /// ```
    /// use druid_shell::{KeyCode, Modifier};
    /// # let key_event = druid_shell::KeyEvent::for_test(Modifier::NoMods, "a", KeyCode::KeyA);
    /// let is_undo = key_event.matches(Modifier::Ctrl + 'z');
    /// let is_reset = key_event.matches(Modifier::Ctrl + Modifier::Alt + KeyCode::Delete);
    /// ```
    pub fn matches(&self, matcher: impl KeyMatcher) -> bool {
        matcher.matches(self)
    }

    /// For creating `KeyEvent`s during testing.
    #[doc(hidden)]
    pub fn for_test(mods: impl Into<KeyModifiers>, text: &'static str, code: KeyCode) -> Self {
        KeyEvent::new(code, false, mods.into(), text, text)
    }
}

/// Keyboard modifier state, provided for events.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct KeyModifiers {
    /// Shift.
    pub shift: bool,
    /// Option on macOS.
    pub alt: bool,
    /// Control.
    pub ctrl: bool,
    /// Meta / Windows / Command
    pub meta: bool,
}

/// Should realistically be (8 * N) - 1; we need one byte for the length.
const TINY_STR_CAPACITY: usize = 15;

/// A stack allocated string with a fixed capacity.
#[derive(Clone, Copy)]
struct TinyStr {
    len: u8,
    buf: [u8; TINY_STR_CAPACITY],
}

impl TinyStr {
    fn new<S: AsRef<str>>(s: S) -> Self {
        let s = s.as_ref();
        let len = match s.len() {
            l @ 0..=15 => l,
            more => {
                // If some user has weird unicode bound to a key, it's better to
                // mishandle that input then to crash the client application?
                debug_assert!(
                    false,
                    "Err 101, Invalid Assumptions: TinyStr::new \
                     called with {} (len {}).",
                    s, more
                );
                #[cfg(test)]
                {
                    // we still want to fail when testing a release build
                    assert!(
                        false,
                        "Err 101, Invalid Assumptions: TinyStr::new \
                         called with {} (len {}).",
                        s, more
                    );
                }

                // rups. find last valid codepoint offset
                let mut len = 15;
                while !s.is_char_boundary(len) {
                    len -= 1;
                }
                len
            }
        };
        let mut buf = [0; 15];
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        TinyStr {
            len: len as u8,
            buf,
        }
    }

    fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

impl From<char> for TinyStr {
    fn from(src: char) -> TinyStr {
        let len = src.len_utf8();
        let mut buf = [0; 15];
        src.encode_utf8(&mut buf);

        TinyStr {
            len: len as u8,
            buf,
        }
    }
}

/// A type we use in the constructor of `KeyEvent`, specifically to avoid exposing
/// internals.
pub enum StrOrChar {
    Char(char),
    Str(&'static str),
}

impl From<&'static str> for StrOrChar {
    fn from(src: &'static str) -> Self {
        StrOrChar::Str(src)
    }
}

impl From<char> for StrOrChar {
    fn from(src: char) -> StrOrChar {
        StrOrChar::Char(src)
    }
}

impl From<Option<char>> for StrOrChar {
    fn from(src: Option<char>) -> StrOrChar {
        match src {
            Some(c) => StrOrChar::Char(c),
            None => StrOrChar::Str(""),
        }
    }
}

impl fmt::Display for TinyStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for TinyStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TinyStr(\"{}\")", self.as_str())
    }
}

impl Default for TinyStr {
    fn default() -> Self {
        TinyStr::new("")
    }
}

#[allow(clippy::useless_let_if_seq)]
impl fmt::Debug for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Mods(")?;
        let mut has_prev = false;
        if self.meta {
            write!(f, "meta")?;
            has_prev = true;
        }
        if self.ctrl {
            if has_prev {
                write!(f, "+")?;
            }
            write!(f, "ctrl")?;
            has_prev = true;
        }
        if self.alt {
            if has_prev {
                write!(f, "+")?;
            }
            write!(f, "alt")?;
            has_prev = true;
        }
        if self.shift {
            if has_prev {
                write!(f, "+")?;
            }
            write!(f, "shift")?;
            has_prev = true;
        }
        if !has_prev {
            write!(f, "None")?;
        }
        write!(f, ")")
    }
}

// Key matcher logic

/// An internal matcher constructor for negating a modifier.
pub struct NotMod(Modifier);

/// An internal matcher constructor for taking the "or" of two matchers.
pub struct OrMatcher<T, U>(T, U);

pub struct AddMatcher<T, U>(T, U);

macro_rules! impl_op {
    ($tr:ident, $f:ident, $comb:ident, $lhs:ident $(< $($ltv:ident),+ >)?,
        $rhs:ident $(< $($rtv:ident),+ >)?) =>
    {
        impl<$($($ltv,)*)? $($($rtv),*)?> std::ops::$tr<$rhs<$($($rtv),+)?>> for
            $lhs$(<$($ltv),*>)?
        {
            type Output = $comb<Self, $rhs $(<$($rtv),+>)?>;
            fn $f(self, rhs: $rhs $(<$($rtv),+>)?) -> Self::Output {
                $comb(self, rhs)
            }
        }
    };
}

macro_rules! impl_adds {
    ($rhs:ident $(< $($rtv:ident),+ >)?) => {
        impl_op!(Add, add, AddMatcher, Modifier, $rhs $(<$($rtv),+>)?);
        impl_op!(Add, add, AddMatcher, NotMod, $rhs $(<$($rtv),+>)?);
        impl_op!(Add, add, AddMatcher, OrMatcher<T0, T1>, $rhs $(<$($rtv),+>)?);
        impl_op!(Add, add, AddMatcher, AddMatcher<T0, T1>, $rhs $(<$($rtv),+>)?);
    };
}

macro_rules! impl_ors {
    ($rhs:ident $(< $($rtv:ident),+ >)?) => {
        impl_op!(BitOr, bitor, OrMatcher, Modifier, $rhs $(<$($rtv),+>)?);
        impl_op!(BitOr, bitor, OrMatcher, NotMod, $rhs $(<$($rtv),+>)?);
        impl_op!(BitOr, bitor, OrMatcher, OrMatcher<T0, T1>, $rhs $(<$($rtv),+>)?);
        impl_op!(BitOr, bitor, OrMatcher, AddMatcher<T0, T1>, $rhs $(<$($rtv),+>)?);
    };
}

impl_adds!(Modifier);
impl_adds!(NotMod);
impl_adds!(OrMatcher<T, U>);
impl_adds!(AddMatcher<T, U>);
impl_ors!(Modifier);
impl_ors!(NotMod);
impl_ors!(OrMatcher<T, U>);
impl_ors!(AddMatcher<T, U>);

impl_adds!(char);
impl_adds!(KeyCode);

impl From<Modifier> for KeyModifiers {
    fn from(src: Modifier) -> KeyModifiers {
        src.to_modifiers()
    }
}

impl Modifier {
    fn to_modifiers(&self) -> KeyModifiers {
        let mut result = KeyModifiers::default();
        match self {
            Modifier::Alt => result.alt = true,
            Modifier::Ctrl => result.ctrl = true,
            Modifier::Meta => result.meta = true,
            Modifier::Shift => result.shift = true,
            Modifier::Cmd => {
                #[cfg(target_os = "macos")]
                {
                    result.meta = true;
                }
                #[cfg(not(target_os = "macos"))]
                {
                    result.ctrl = true;
                }
            }
            Modifier::NoMods => (),
        }
        result
    }
}

impl ModMatcher for Modifier {
    fn match_mods(&self, mods: KeyModifiers) -> Option<KeyModifiers> {
        match *self {
            Modifier::Alt => {
                if !mods.alt {
                    return None;
                }
            }
            Modifier::Ctrl => {
                if !mods.ctrl {
                    return None;
                }
            }
            Modifier::Meta => {
                if !mods.meta {
                    return None;
                }
            }
            Modifier::Shift => {
                if !mods.shift {
                    return None;
                }
            }
            Modifier::Cmd => {
                #[cfg(target_os = "macos")]
                {
                    if !mods.meta {
                        return None;
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    if !mods.ctrl {
                        return None;
                    }
                }
            }
            Modifier::NoMods => {
                if mods.alt || mods.ctrl || mods.meta || mods.shift {
                    return None;
                }
            }
        }
        Some(self.to_modifiers())
    }
}

impl std::ops::Not for Modifier {
    type Output = NotMod;
    fn not(self) -> NotMod {
        NotMod(self)
    }
}

impl ModMatcher for NotMod {
    fn match_mods(&self, mods: KeyModifiers) -> Option<KeyModifiers> {
        match self.0 {
            Modifier::Alt => {
                if mods.alt {
                    return None;
                }
            }
            Modifier::Ctrl => {
                if mods.ctrl {
                    return None;
                }
            }
            Modifier::Meta => {
                if mods.meta {
                    return None;
                }
            }
            Modifier::Shift => {
                if mods.shift {
                    return None;
                }
            }
            Modifier::Cmd => {
                #[cfg(target_os = "macos")]
                {
                    if mods.meta {
                        return None;
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    if mods.ctrl {
                        return None;
                    }
                }
            }
            Modifier::NoMods => {
                if !(mods.alt || mods.ctrl || mods.meta || mods.shift) {
                    return None;
                } else {
                    // Make it so !NoMods matches all non-empty modifier sets.
                    return Some(KeyModifiers {
                        alt: true,
                        ctrl: true,
                        meta: true,
                        shift: true,
                    });
                }
            }
        }
        Some(self.0.to_modifiers())
    }
}

impl<T: ModMatcher, U: ModMatcher> ModMatcher for OrMatcher<T, U> {
    fn match_mods(&self, mods: KeyModifiers) -> Option<KeyModifiers> {
        self.0.match_mods(mods).or_else(|| self.1.match_mods(mods))
    }
}

impl<T: ModMatcher, U: ModMatcher> ModMatcher for AddMatcher<T, U> {
    fn match_mods(&self, mods: KeyModifiers) -> Option<KeyModifiers> {
        if let Some(covered0) = self.0.match_mods(mods) {
            if let Some(covered1) = self.1.match_mods(mods) {
                // Bitwise would be nicer than struct-of-bools here.
                return Some(KeyModifiers {
                    alt: covered0.alt | covered1.alt,
                    ctrl: covered0.ctrl | covered1.ctrl,
                    meta: covered0.meta | covered1.meta,
                    shift: covered0.shift | covered1.shift,
                });
            }
        }
        None
    }
}

impl KeyMatcher for char {
    fn matches_internal(&self, key_event: &KeyEvent, covered: KeyModifiers) -> bool {
        // TODO: make "shift" handling more sophisticated. Currently, shift is allowed,
        // because some characters need it. However, this means that Control-Shift-Z
        // matches a spec of `Control + 'z'`, and that probably shouldn't be. To get
        // more specificity, use `Control + !Shift + 'z'` for now.
        if (key_event.mods.alt && !covered.alt)
            || (key_event.mods.ctrl && !covered.ctrl)
            || (key_event.mods.meta && !covered.meta)
        {
            return false;
        }
        // We allow either text or unmodified text to match.
        let text = key_event.text().unwrap_or("");
        let mut chars = text.chars();
        if chars.next() == Some(*self) && chars.next().is_none() {
            return true;
        }
        let unmod_text = key_event.unmod_text().unwrap_or("");
        let mut chars = unmod_text.chars();
        if chars.next() == Some(*self) && chars.next().is_none() {
            return true;
        }
        false
    }
}

impl KeyMatcher for KeyCode {
    fn matches_internal(&self, key_event: &KeyEvent, covered: KeyModifiers) -> bool {
        if (key_event.mods.alt && !covered.alt)
            || (key_event.mods.ctrl && !covered.ctrl)
            || (key_event.mods.meta && !covered.meta)
            || (key_event.mods.shift && !covered.shift)
        {
            return false;
        }
        key_event.key_code == *self
    }
}

impl<T: ModMatcher, U: KeyMatcher> KeyMatcher for AddMatcher<T, U> {
    fn matches(&self, key_event: &KeyEvent) -> bool {
        if let Some(covered) = self.0.match_mods(key_event.mods) {
            self.1.matches_internal(key_event, covered)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_panic(expected = "Invalid Assumptions")]
    fn smallstr() {
        let smol = TinyStr::new("hello");
        assert_eq!(smol.as_str(), "hello");
        let smol = TinyStr::new("");
        assert_eq!(smol.as_str(), "");
        let s_16 = "😍🥰😘😗";
        assert_eq!(s_16.len(), 16);
        assert!(!s_16.is_char_boundary(15));
        let _too_big = TinyStr::new("😍🥰😘😗");
    }

    #[test]
    fn key_matching() {
        use super::Modifier::*;
        let key = KeyEvent::for_test(NoMods, "a", KeyCode::KeyA);
        assert!(key.matches('a'));
        assert!(!key.matches('b'));
        assert!(key.matches(KeyCode::KeyA));
        assert!(!key.matches(KeyCode::KeyB));
        assert!(key.matches(!Shift + 'a'));
        assert!(!key.matches(Shift + 'a'));

        let ctrl_alt = KeyModifiers {
            alt: true,
            ctrl: true,
            ..Default::default()
        };
        let tfs = KeyEvent::for_test(ctrl_alt, "", KeyCode::Delete);
        assert!(tfs.matches(Ctrl + Alt + KeyCode::Delete));
        assert!(tfs.matches(Ctrl + (Alt | Meta) + KeyCode::Delete));
        assert!(!tfs.matches(KeyCode::Delete));
    }
}
