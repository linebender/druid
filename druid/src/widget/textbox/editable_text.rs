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

use std::ops::Range;
use unicode_segmentation::GraphemeCursor;

pub trait EditableText: Sized {
    type Cursor: EditableTextCursor<Self>;

    fn cursor(&self, position: usize) -> Self::Cursor;

    //TODO: this can panic so maybe we should do a result??
    fn edit(&mut self, iv: Range<usize>, new: impl Into<String>);

    fn slice(&self, iv: Range<usize>) -> Option<&str>;

    fn len(&self) -> usize;

    // pub fn is_codepoint_boundary(&self, offset: usize) -> bool;

    // fn prev_codepoint_offset(&self, offset: usize) -> Option<usize>;

    // fn next_codepoint_offset(&self, offset: usize) -> Option<usize>;

    // pub fn at_or_next_codepoint_boundary(&self, offset: usize) -> Option<usize>;

    // pub fn at_or_prev_codepoint_boundary(&self, offset: usize) -> Option<usize>;

    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize>;

    fn next_grapheme_offset(&self, offset: usize) -> Option<usize>;

    fn is_empty(&self) -> bool;
}

impl EditableText for String {
    type Cursor = StringCursor;

    fn edit(&mut self, iv: Range<usize>, new: impl Into<String>) {
        self.replace_range(iv, &new.into());
    }

    fn slice(&self, iv: Range<usize>) -> Option<&str> {
        self.get(iv)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn cursor(&self, position: usize) -> StringCursor {
        StringCursor {
            text: self.to_string(),
            position,
        }
    }
    /// Gets the next grapheme from the given index.
    fn next_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        c.next_boundary(self, 0).unwrap()
    }

    /// Gets the previous grapheme from the given index.
    fn prev_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        c.prev_boundary(self, 0).unwrap()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

pub trait EditableTextCursor<EditableText> {
    // fn new(s: EditableText, position: usize) -> Self;

    // fn total_len(&self) -> usize;

    // fn text(&self) -> &str;

    // set cursor position
    fn set(&mut self, position: usize);

    // get cursor position
    fn pos(&self) -> usize;

    fn is_boundary(&self) -> bool;

    // moves cursor to previous boundary if exists
    // else becomes invalid cursor
    fn prev(&mut self) -> Option<usize>;

    fn next(&mut self) -> Option<usize>;

    fn prev_codepoint(&mut self) -> Option<char>;
    fn next_codepoint(&mut self) -> Option<char>;

    //return current if it's a boundary, else next
    fn at_or_next(&mut self) -> Option<usize>;

    fn at_or_prev(&mut self) -> Option<usize>;

    // pub fn iter(&mut self) -> CursorIter???
}

pub struct StringCursor {
    text: String,
    position: usize,
}

impl EditableTextCursor<String> for StringCursor {
    // fn new(text: &'a str, position: usize) -> StringCursor<'a> {
    //     StringCursor { text, position }
    // }

    // fn total_len(&self) -> usize {
    //     self.text.len()
    // }

    // fn text(&self) -> &str {
    //     self.text
    // }

    // set cursor position
    fn set(&mut self, position: usize) {
        self.position = position;
    }

    // get cursor position
    fn pos(&self) -> usize {
        self.position
    }

    fn is_boundary(&self) -> bool {
        self.text.is_char_boundary(self.position)
    }

    // moves cursor to previous boundary if exists
    // else becomes invalid cursor
    fn prev(&mut self) -> Option<usize> {
        let current_index = self.pos();
        if current_index == 0 {
            return None;
        }

        // This seems wasteful but I don't have a "chunk" concept to help out
        let (prev_index, _) = self.text[..current_index]
            .char_indices()
            .rev()
            .next()
            .unwrap();

        // TODO: is this correct?
        self.set(prev_index);
        Some(self.pos())
    }

    fn next(&mut self) -> Option<usize> {
        let current_index = self.pos();
        if current_index == self.text.len() {
            return None;
        }

        // This seems wasteful but I don't have a "chunk" concept to help out
        let (next_index, _) = self.text[current_index..].char_indices().next().unwrap();

        // TODO: is this correct?
        self.set(next_index);
        Some(self.pos())
    }

    fn prev_codepoint(&mut self) -> Option<char> {
        // WARNING: .prev() mutates!
        if let Some(prev) = self.prev() {
            self.text.chars().nth(prev)
        } else {
            None
        }
    }
    fn next_codepoint(&mut self) -> Option<char> {
        // WARNING: .next() mutates!
        if let Some(next) = self.next() {
            self.text.chars().nth(next)
        } else {
            None
        }
    }

    //return current if it's a boundary, else next
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
