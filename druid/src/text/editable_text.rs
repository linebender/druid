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

//! Traits for text editing and a basic String implementation.

use std::borrow::Cow;
use std::ops::Range;

use unicode_segmentation::GraphemeCursor;

/// An EditableText trait.
pub trait EditableText: Sized {
    // TODO: would be nice to have something like
    // type Cursor: EditableTextCursor<Self>;

    /// Create a cursor with a reference to the text and a offset position.
    ///
    /// Returns None if the position isn't a codepoint boundary.
    fn cursor(&self, position: usize) -> Option<StringCursor>;

    /// Replace range with new text.
    /// Can panic if supplied an invalid range.
    // TODO: make this generic over Self
    fn edit(&mut self, range: Range<usize>, new: impl Into<String>);

    /// Get slice of text at range.
    fn slice(&self, range: Range<usize>) -> Option<Cow<str>>;

    /// Get length of text (in bytes).
    fn len(&self) -> usize;

    /// Get the next grapheme offset from the given offset, if it exists.
    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize>;

    /// Get the previous grapheme offset from the given offset, if it exists.
    fn next_grapheme_offset(&self, offset: usize) -> Option<usize>;

    /// Get the next codepoint offset from the given offset, if it exists.
    fn prev_codepoint_offset(&self, offset: usize) -> Option<usize>;

    /// Get the previous codepoint offset from the given offset, if it exists.
    fn next_codepoint_offset(&self, offset: usize) -> Option<usize>;

    fn is_empty(&self) -> bool;

    fn from_str(s: &str) -> Self;
}

impl EditableText for String {
    fn cursor<'a>(&self, position: usize) -> Option<StringCursor> {
        let new_cursor = StringCursor {
            text: &self,
            position,
        };

        if new_cursor.is_boundary() {
            Some(new_cursor)
        } else {
            None
        }
    }

    fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
        self.replace_range(range, &new.into());
    }

    fn slice(&self, range: Range<usize>) -> Option<Cow<str>> {
        if let Some(slice) = self.get(range) {
            Some(Cow::from(slice))
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn prev_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        c.prev_boundary(self, 0).unwrap()
    }

    fn next_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        c.next_boundary(self, 0).unwrap()
    }

    fn prev_codepoint_offset(&self, from: usize) -> Option<usize> {
        let mut c = self.cursor(from).unwrap();
        c.prev()
    }

    fn next_codepoint_offset(&self, from: usize) -> Option<usize> {
        let mut c = self.cursor(from).unwrap();
        if c.next().is_some() {
            Some(c.pos())
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn from_str(s: &str) -> Self {
        s.to_string()
    }
}

/// A cursor with convenience functions for moving through EditableText.
pub trait EditableTextCursor<EditableText> {
    /// Set cursor position.
    fn set(&mut self, position: usize);

    /// Get cursor position.
    fn pos(&self) -> usize;

    /// Check if cursor position is at a codepoint boundary.
    fn is_boundary(&self) -> bool;

    /// Move cursor to previous codepoint boundary, if it exists.
    /// Returns previous codepoint as usize offset.
    fn prev(&mut self) -> Option<usize>;

    /// Move cursor to next codepoint boundary, if it exists.
    /// Returns current codepoint as usize offset.
    fn next(&mut self) -> Option<usize>;

    /// Get the next codepoint after the cursor position, without advancing
    /// the cursor.
    fn peek_next_codepoint(&self) -> Option<char>;

    /// Return codepoint preceding cursor offset and move cursor backward.
    fn prev_codepoint(&mut self) -> Option<char>;

    /// Return codepoint at cursor offset and move cursor forward.
    fn next_codepoint(&mut self) -> Option<char>;

    /// Return current offset if it's a boundary, else next.
    fn at_or_next(&mut self) -> Option<usize>;

    /// Return current offset if it's a boundary, else previous.
    fn at_or_prev(&mut self) -> Option<usize>;
}

/// A cursor type that implements EditableTextCursor for String
#[derive(Debug)]
pub struct StringCursor<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> EditableTextCursor<&'a String> for StringCursor<'a> {
    fn set(&mut self, position: usize) {
        self.position = position;
    }

    fn pos(&self) -> usize {
        self.position
    }

    fn is_boundary(&self) -> bool {
        self.text.is_char_boundary(self.position)
    }

    fn prev(&mut self) -> Option<usize> {
        let current_pos = self.pos();

        if current_pos == 0 {
            None
        } else {
            let mut len = 1;
            while !self.text.is_char_boundary(current_pos - len) {
                len += 1;
            }
            self.set(self.pos() - len);
            Some(self.pos())
        }
    }

    fn next(&mut self) -> Option<usize> {
        let current_pos = self.pos();

        if current_pos == self.text.len() {
            None
        } else {
            let b = self.text.as_bytes()[current_pos];
            self.set(current_pos + len_utf8_from_first_byte(b));
            Some(current_pos)
        }
    }

    fn peek_next_codepoint(&self) -> Option<char> {
        self.text[self.pos()..].chars().next()
    }

    fn prev_codepoint(&mut self) -> Option<char> {
        if let Some(prev) = self.prev() {
            self.text[prev..].chars().next()
        } else {
            None
        }
    }

    fn next_codepoint(&mut self) -> Option<char> {
        let current_index = self.pos();
        if self.next().is_some() {
            self.text[current_index..].chars().next()
        } else {
            None
        }
    }

    fn at_or_next(&mut self) -> Option<usize> {
        if self.is_boundary() {
            Some(self.pos())
        } else {
            self.next()
        }
    }

    fn at_or_prev(&mut self) -> Option<usize> {
        if self.is_boundary() {
            Some(self.pos())
        } else {
            self.prev()
        }
    }
}

pub fn len_utf8_from_first_byte(b: u8) -> usize {
    match b {
        b if b < 0x80 => 1,
        b if b < 0xe0 => 2,
        b if b < 0xf0 => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace() {
        let mut a = String::from("hello world");
        a.edit(1..9, "era");
        assert_eq!("herald", a);
    }

    #[test]
    fn prev_codepoint_offset() {
        let a = String::from("a\u{00A1}\u{4E00}\u{1F4A9}");
        assert_eq!(Some(6), a.prev_codepoint_offset(10));
        assert_eq!(Some(3), a.prev_codepoint_offset(6));
        assert_eq!(Some(1), a.prev_codepoint_offset(3));
        assert_eq!(Some(0), a.prev_codepoint_offset(1));
        assert_eq!(None, a.prev_codepoint_offset(0));
        let b = a.slice(1..10).unwrap().to_string();
        assert_eq!(Some(5), b.prev_codepoint_offset(9));
        assert_eq!(Some(2), b.prev_codepoint_offset(5));
        assert_eq!(Some(0), b.prev_codepoint_offset(2));
        assert_eq!(None, b.prev_codepoint_offset(0));
    }

    #[test]
    fn next_codepoint_offset() {
        let a = String::from("a\u{00A1}\u{4E00}\u{1F4A9}");
        assert_eq!(Some(10), a.next_codepoint_offset(6));
        assert_eq!(Some(6), a.next_codepoint_offset(3));
        assert_eq!(Some(3), a.next_codepoint_offset(1));
        assert_eq!(Some(1), a.next_codepoint_offset(0));
        assert_eq!(None, a.next_codepoint_offset(10));
        let b = a.slice(1..10).unwrap().to_string();
        assert_eq!(Some(9), b.next_codepoint_offset(5));
        assert_eq!(Some(5), b.next_codepoint_offset(2));
        assert_eq!(Some(2), b.next_codepoint_offset(0));
        assert_eq!(None, b.next_codepoint_offset(9));
    }

    #[test]
    fn prev_next() {
        let input = String::from("abc");
        let mut cursor = input.cursor(0).unwrap();
        assert_eq!(cursor.next(), Some(0));
        assert_eq!(cursor.next(), Some(1));
        assert_eq!(cursor.prev(), Some(1));
        assert_eq!(cursor.next(), Some(1));
        assert_eq!(cursor.next(), Some(2));
    }

    #[test]
    fn peek_next_codepoint() {
        let inp = String::from("$¢€£💶");
        let mut cursor = inp.cursor(0).unwrap();
        assert_eq!(cursor.peek_next_codepoint(), Some('$'));
        assert_eq!(cursor.peek_next_codepoint(), Some('$'));
        assert_eq!(cursor.next_codepoint(), Some('$'));
        assert_eq!(cursor.peek_next_codepoint(), Some('¢'));
        assert_eq!(cursor.prev_codepoint(), Some('$'));
        assert_eq!(cursor.peek_next_codepoint(), Some('$'));
        assert_eq!(cursor.next_codepoint(), Some('$'));
        assert_eq!(cursor.next_codepoint(), Some('¢'));
        assert_eq!(cursor.peek_next_codepoint(), Some('€'));
        assert_eq!(cursor.next_codepoint(), Some('€'));
        assert_eq!(cursor.peek_next_codepoint(), Some('£'));
        assert_eq!(cursor.next_codepoint(), Some('£'));
        assert_eq!(cursor.peek_next_codepoint(), Some('💶'));
        assert_eq!(cursor.next_codepoint(), Some('💶'));
        assert_eq!(cursor.peek_next_codepoint(), None);
        assert_eq!(cursor.next_codepoint(), None);
        assert_eq!(cursor.peek_next_codepoint(), None);
    }

    #[test]
    fn prev_grapheme_offset() {
        // A with ring, hangul, regional indicator "US"
        let a = String::from("A\u{030a}\u{110b}\u{1161}\u{1f1fa}\u{1f1f8}");
        assert_eq!(Some(9), a.prev_grapheme_offset(17));
        assert_eq!(Some(3), a.prev_grapheme_offset(9));
        assert_eq!(Some(0), a.prev_grapheme_offset(3));
        assert_eq!(None, a.prev_grapheme_offset(0));
    }

    #[test]
    fn next_grapheme_offset() {
        // A with ring, hangul, regional indicator "US"
        let a = String::from("A\u{030a}\u{110b}\u{1161}\u{1f1fa}\u{1f1f8}");
        assert_eq!(Some(3), a.next_grapheme_offset(0));
        assert_eq!(Some(9), a.next_grapheme_offset(3));
        assert_eq!(Some(17), a.next_grapheme_offset(9));
        assert_eq!(None, a.next_grapheme_offset(17));
    }
}
