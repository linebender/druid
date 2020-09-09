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

//! Text editing movements.

use crate::text::{EditableText, Selection};

/// The specification of a movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Movement {
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
    /// Move to the left by one word.
    LeftWord,
    /// Move to the right by one word.
    RightWord,
    /// Move to left end of visible line.
    PrecedingLineBreak,
    /// Move to right end of visible line.
    NextLineBreak,
    /// Move to the beginning of the document
    StartOfDocument,
    /// Move to the end of the document
    EndOfDocument,
}

/// Compute the result of movement on a selection .
pub fn movement(m: Movement, s: Selection, text: &impl EditableText, modify: bool) -> Selection {
    let offset = match m {
        Movement::Left => {
            if s.is_caret() || modify {
                text.prev_grapheme_offset(s.end).unwrap_or(0)
            } else {
                s.min()
            }
        }
        Movement::Right => {
            if s.is_caret() || modify {
                text.next_grapheme_offset(s.end).unwrap_or(s.end)
            } else {
                s.max()
            }
        }

        Movement::PrecedingLineBreak => text.preceding_line_break(s.end),
        Movement::NextLineBreak => text.next_line_break(s.end),

        Movement::StartOfDocument => 0,
        Movement::EndOfDocument => text.len(),

        Movement::LeftWord => {
            if s.is_caret() || modify {
                text.prev_word_offset(s.end).unwrap_or(0)
            } else {
                s.min()
            }
        }
        Movement::RightWord => {
            if s.is_caret() || modify {
                text.next_word_offset(s.end).unwrap_or(s.end)
            } else {
                s.max()
            }
        }
    };
    Selection::new(if modify { s.start } else { offset }, offset)
}
