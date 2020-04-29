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

use std::fmt;

use super::keycodes::KeyCode;

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

impl KeyEvent {
    /// Create a new `KeyEvent` struct. This accepts either &str or char for the last
    /// two arguments.
    pub(crate) fn new<'a>(
        key_code: impl Into<KeyCode>,
        is_repeat: bool,
        mods: KeyModifiers,
        text: impl Into<StrOrChar<'a>>,
        unmodified_text: impl Into<StrOrChar<'a>>,
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
pub enum StrOrChar<'a> {
    Char(char),
    Str(&'a str),
}

impl<'a> From<&'a str> for StrOrChar<'a> {
    fn from(src: &'a str) -> Self {
        StrOrChar::Str(src)
    }
}

impl From<char> for StrOrChar<'static> {
    fn from(src: char) -> StrOrChar<'static> {
        StrOrChar::Char(src)
    }
}

impl From<Option<char>> for StrOrChar<'static> {
    fn from(src: Option<char>) -> StrOrChar<'static> {
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
}
