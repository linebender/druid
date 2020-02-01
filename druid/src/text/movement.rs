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

//! Text editing movements.

use crate::text::{EditableText, Selection};

/// The specification of a movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Movement {
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
    /// Move to left end of visible line.
    LeftOfLine,
    /// Move to right end of visible line.
    RightOfLine,
}

/// Compute the result of movement on a selection .
pub fn movement(m: Movement, s: Selection, text: &impl EditableText, modify: bool) -> Selection {
    let offset = match m {
        Movement::Left => {
            if s.is_caret() || modify {
                if let Some(offset) = text.prev_grapheme_offset(s.end) {
                    offset
                } else {
                    0
                }
            } else {
                s.min()
            }
        }
        Movement::Right => {
            if s.is_caret() || modify {
                if let Some(offset) = text.next_grapheme_offset(s.end) {
                    offset
                } else {
                    s.end
                }
            } else {
                s.max()
            }
        }

        Movement::LeftOfLine => 0,
        Movement::RightOfLine => text.len(),
    };
    Selection::new(if modify { s.start } else { offset }, offset)
}
