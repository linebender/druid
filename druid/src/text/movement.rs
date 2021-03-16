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

use crate::kurbo::Point;
use crate::piet::TextLayout as _;
use crate::text::{EditableText, Selection, TextLayout, TextStorage};

/// The specification of a movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Movement {
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
    /// Move up one visible line.
    Up,
    /// Move down one visible line.
    Down,
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

//FIXME: we should remove this whole file, and use the Movement type defined in druid-shell?
impl From<crate::shell::text::Movement> for Movement {
    fn from(src: crate::shell::text::Movement) -> Movement {
        use crate::shell::text::{Direction, Movement as SMovemement, VerticalMovement};
        match src {
            SMovemement::Grapheme(Direction::Left) | SMovemement::Grapheme(Direction::Upstream) => {
                Movement::Left
            }
            SMovemement::Grapheme(_) => Movement::Right,
            SMovemement::Word(Direction::Left) => Movement::LeftWord,
            SMovemement::Word(_) => Movement::RightWord,
            SMovemement::Line(Direction::Left) | SMovemement::ParagraphStart => {
                Movement::PrecedingLineBreak
            }
            SMovemement::Line(_) | SMovemement::ParagraphEnd => Movement::NextLineBreak,
            SMovemement::Vertical(VerticalMovement::LineUp)
            | SMovemement::Vertical(VerticalMovement::PageUp) => Movement::Up,
            SMovemement::Vertical(VerticalMovement::LineDown)
            | SMovemement::Vertical(VerticalMovement::PageDown) => Movement::Down,
            SMovemement::Vertical(VerticalMovement::DocumentStart) => Movement::StartOfDocument,
            SMovemement::Vertical(VerticalMovement::DocumentEnd) => Movement::EndOfDocument,
            // the source enum is non_exhaustive
            _ => panic!("unhandled movement {:?}", src),
        }
    }
}

/// Compute the result of movement on a selection.
///
/// returns a new selection representing the state after the movement.
///
/// If `modify` is true, only the 'active' edge (the `end`) of the selection
/// should be changed; this is the case when the user moves with the shift
/// key pressed.
pub fn movement<T: EditableText + TextStorage>(
    m: Movement,
    s: Selection,
    layout: &TextLayout<T>,
    modify: bool,
) -> Selection {
    let (text, layout) = match (layout.text(), layout.layout()) {
        (Some(text), Some(layout)) => (text, layout),
        _ => {
            debug_assert!(false, "movement() called before layout rebuild");
            return s;
        }
    };

    let (offset, h_pos) = match m {
        Movement::Left => {
            if s.is_caret() || modify {
                text.prev_grapheme_offset(s.active)
                    .map(|off| (off, None))
                    .unwrap_or((0, s.h_pos))
            } else {
                (s.min(), None)
            }
        }
        Movement::Right => {
            if s.is_caret() || modify {
                text.next_grapheme_offset(s.active)
                    .map(|off| (off, None))
                    .unwrap_or((s.active, s.h_pos))
            } else {
                (s.max(), None)
            }
        }

        Movement::Up => {
            let cur_pos = layout.hit_test_text_position(s.active);
            let h_pos = s.h_pos.unwrap_or(cur_pos.point.x);
            if cur_pos.line == 0 {
                (0, Some(h_pos))
            } else {
                let lm = layout.line_metric(cur_pos.line).unwrap();
                let point_above = Point::new(h_pos, cur_pos.point.y - lm.height);
                let up_pos = layout.hit_test_point(point_above);
                (up_pos.idx, Some(point_above.x))
            }
        }
        Movement::Down => {
            let cur_pos = layout.hit_test_text_position(s.active);
            let h_pos = s.h_pos.unwrap_or(cur_pos.point.x);
            if cur_pos.line == layout.line_count() - 1 {
                (text.len(), Some(h_pos))
            } else {
                let lm = layout.line_metric(cur_pos.line).unwrap();
                // may not work correctly for point sizes below 1.0
                let y_below = lm.y_offset + lm.height + 1.0;
                let point_below = Point::new(h_pos, y_below);
                let up_pos = layout.hit_test_point(point_below);
                (up_pos.idx, Some(point_below.x))
            }
        }

        Movement::PrecedingLineBreak => (text.preceding_line_break(s.active), None),
        Movement::NextLineBreak => (text.next_line_break(s.active), None),

        Movement::StartOfDocument => (0, None),
        Movement::EndOfDocument => (text.len(), None),

        Movement::LeftWord => {
            let offset = if s.is_caret() || modify {
                text.prev_word_offset(s.active).unwrap_or(0)
            } else {
                s.min()
            };
            (offset, None)
        }
        Movement::RightWord => {
            let offset = if s.is_caret() || modify {
                text.next_word_offset(s.active).unwrap_or(s.active)
            } else {
                s.max()
            };
            (offset, None)
        }
    };

    let start = if modify { s.anchor } else { offset };
    Selection::new(start, offset).with_h_pos(h_pos)
}
