// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Text editing movements.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::kurbo::Point;
use crate::piet::TextLayout as _;
use crate::text::{
    EditableText, Movement, Selection, TextLayout, TextStorage, VerticalMovement, WritingDirection,
};

/// Compute the result of a [`Movement`] on a [`Selection`].
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

    let writing_direction = if crate::piet::util::first_strong_rtl(text.as_str()) {
        WritingDirection::RightToLeft
    } else {
        WritingDirection::LeftToRight
    };

    let (offset, h_pos) = match m {
        Movement::Grapheme(d) if d.is_upstream_for_direction(writing_direction) => {
            if s.is_caret() || modify {
                text.prev_grapheme_offset(s.active)
                    .map(|off| (off, None))
                    .unwrap_or((0, s.h_pos))
            } else {
                (s.min(), None)
            }
        }
        Movement::Grapheme(_) => {
            if s.is_caret() || modify {
                text.next_grapheme_offset(s.active)
                    .map(|off| (off, None))
                    .unwrap_or((s.active, s.h_pos))
            } else {
                (s.max(), None)
            }
        }
        Movement::Vertical(VerticalMovement::LineUp) => {
            let cur_pos = layout.hit_test_text_position(s.active);
            let h_pos = s.h_pos.unwrap_or(cur_pos.point.x);
            if cur_pos.line == 0 {
                (0, Some(h_pos))
            } else {
                let lm = layout.line_metric(cur_pos.line).unwrap();
                let point_above = Point::new(h_pos, cur_pos.point.y - lm.height);
                let up_pos = layout.hit_test_point(point_above);
                if up_pos.is_inside {
                    (up_pos.idx, Some(h_pos))
                } else {
                    // because we can't specify affinity, moving up when h_pos
                    // is wider than both the current line and the previous line
                    // can result in a cursor position at the visual start of the
                    // current line; so we handle this as a special-case.
                    let lm_prev = layout.line_metric(cur_pos.line.saturating_sub(1)).unwrap();
                    let up_pos = lm_prev.end_offset - lm_prev.trailing_whitespace;
                    (up_pos, Some(h_pos))
                }
            }
        }
        Movement::Vertical(VerticalMovement::LineDown) => {
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
        Movement::Vertical(VerticalMovement::DocumentStart) => (0, None),
        Movement::Vertical(VerticalMovement::DocumentEnd) => (text.len(), None),

        Movement::ParagraphStart => (text.preceding_line_break(s.active), None),
        Movement::ParagraphEnd => (text.next_line_break(s.active), None),

        Movement::Line(d) => {
            let hit = layout.hit_test_text_position(s.active);
            let lm = layout.line_metric(hit.line).unwrap();
            let offset = if d.is_upstream_for_direction(writing_direction) {
                lm.start_offset
            } else {
                lm.end_offset - lm.trailing_whitespace
            };
            (offset, None)
        }
        Movement::Word(d) if d.is_upstream_for_direction(writing_direction) => {
            let offset = if s.is_caret() || modify {
                text.prev_word_offset(s.active).unwrap_or(0)
            } else {
                s.min()
            };
            (offset, None)
        }
        Movement::Word(_) => {
            let offset = if s.is_caret() || modify {
                text.next_word_offset(s.active).unwrap_or(s.active)
            } else {
                s.max()
            };
            (offset, None)
        }

        // These two are not handled; they require knowledge of the size
        // of the viewport.
        Movement::Vertical(VerticalMovement::PageDown)
        | Movement::Vertical(VerticalMovement::PageUp) => (s.active, s.h_pos),
        other => {
            tracing::warn!("unhandled movement {:?}", other);
            (s.anchor, s.h_pos)
        }
    };

    let start = if modify { s.anchor } else { offset };
    Selection::new(start, offset).with_h_pos(h_pos)
}

/// Given a position in some text, return the containing word boundaries.
///
/// The returned range may not necessary be a 'word'; for instance it could be
/// the sequence of whitespace between two words.
///
/// If the position is on a word boundary, that will be considered the start
/// of the range.
///
/// This uses Unicode word boundaries, as defined in [UAX#29].
///
/// [UAX#29]: http://www.unicode.org/reports/tr29/
pub(crate) fn word_range_for_pos(text: &str, pos: usize) -> Range<usize> {
    text.split_word_bound_indices()
        .map(|(ix, word)| ix..(ix + word.len()))
        .find(|range| range.contains(&pos))
        .unwrap_or(pos..pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_range_simple() {
        assert_eq!(word_range_for_pos("hello world", 3), 0..5);
        assert_eq!(word_range_for_pos("hello world", 8), 6..11);
    }

    #[test]
    fn word_range_whitespace() {
        assert_eq!(word_range_for_pos("hello world", 5), 5..6);
    }

    #[test]
    fn word_range_rtl() {
        let rtl = "مرحبا بالعالم";
        assert_eq!(word_range_for_pos(rtl, 5), 0..10);
        assert_eq!(word_range_for_pos(rtl, 16), 11..25);
        assert_eq!(word_range_for_pos(rtl, 10), 10..11);
    }

    #[test]
    fn word_range_mixed() {
        let mixed = "hello مرحبا بالعالم world";
        assert_eq!(word_range_for_pos(mixed, 3), 0..5);
        assert_eq!(word_range_for_pos(mixed, 8), 6..16);
        assert_eq!(word_range_for_pos(mixed, 19), 17..31);
        assert_eq!(word_range_for_pos(mixed, 36), 32..37);
    }
}
