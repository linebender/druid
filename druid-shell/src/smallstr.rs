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

//! A simple stack string implementation for key events.

/// A stack allocated string.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SmallStr {
    pub(crate) len: u8,
    pub(crate) buf: [u8; 15],
}

impl SmallStr {
    pub(crate) fn new<S: AsRef<str>>(s: S) -> Self {
        let s = s.as_ref();
        let len = match s.len() {
            l @ 0..=15 => l,
            more => {
                debug_assert!(
                    false,
                    "Err 101, Invalid Assumptions: SmallStr::new called with {} (len {}).",
                    s, more
                );
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
        SmallStr {
            len: len as u8,
            buf,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[should_panic(expected("Invalid Assumptions"))]
    fn smallstr() {
        let smol = SmallStr::new("hello");
        assert_eq!(smol.as_str(), "hello");
        let smol = SmallStr::new("");
        assert_eq!(smol.as_str(), "");
        let s_16 = "😍🥰😘😗";
        assert_eq!(s_16.len(), 16);
        assert!(!s_16.is_char_boundary(15));
        let _too_big = SmallStr::new("😍🥰😘😗");
    }
}
