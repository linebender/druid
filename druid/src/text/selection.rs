// Copyright 2018 The Druid Authors.
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

//! A Selection type for text editing.

use std::cmp::{max, min};
use std::ops::Range;

use crate::text::EditableText;

/// A Selection type for EditableText.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    /// The inactive edge of a selection, as a byte offset. When
    /// equal to end, the selection range acts as a caret.
    pub start: usize,

    /// The active edge of a selection, as a byte offset.
    pub end: usize,
}

impl Selection {
    /// Create a selection that begins at start and goes to end.
    /// Like dragging a mouse from start to end.
    pub fn new(start: usize, end: usize) -> Self {
        Selection { start, end }
    }

    /// Create a selection that starts at the beginning and ends at text length.
    /// TODO: can text length be at a non-codepoint or a non-grapheme?
    pub fn all(&mut self, text: &impl EditableText) {
        self.start = 0;
        self.end = text.len();
    }

    /// Create a caret, which is just a selection with the same and start and end.
    pub fn caret(pos: usize) -> Self {
        Selection {
            start: pos,
            end: pos,
        }
    }

    /// If start == end, it's a caret
    pub fn is_caret(self) -> bool {
        self.start == self.end
    }

    /// Return the smallest index (left, in left-to-right languages)
    pub fn min(self) -> usize {
        min(self.start, self.end)
    }

    /// Return the largest index (right, in left-to-right languages)
    pub fn max(self) -> usize {
        max(self.start, self.end)
    }

    /// Return a range from smallest to largest index
    pub fn range(self) -> Range<usize> {
        self.min()..self.max()
    }

    /// Constrain selection to be not greater than input string
    pub fn constrain_to(mut self, s: &impl EditableText) -> Self {
        let s_len = s.len();
        self.start = min(self.start, s_len);
        self.end = min(self.end, s_len);
        self
    }
}
