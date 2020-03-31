// Copyright 2016 The xi-editor Authors.
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
#![allow(clippy::range_plus_one)]

use std::cmp::max;
use std::ops::Range;

use super::edit_types::{GestureType, SelectionGranularity, ViewEvent};
use super::lines::Lines;
use super::movement::{region_movement, selection_movement, Movement};
use super::selection::{Affinity, InsertDrift, SelRegion, Selection};
use super::word_boundaries::WordCursor;
use xi_rope::{Cursor, Interval, LinesMetric, Rope, RopeDelta};

/// A flag used to indicate when legacy actions should modify selections
const FLAG_SELECT: u64 = 2;

/// Size of batches as number of bytes used during incremental find.
const FIND_BATCH_SIZE: usize = 500000;

pub struct View {
    /// The selection state for this view. Invariant: non-empty.
    selection: Selection,
    drag_state: Option<DragState>,
    lines: Lines,
}

/// State required to resolve a drag gesture into a selection.
struct DragState {
    /// All the selection regions other than the one being dragged.
    base_sel: Selection,

    /// Start of the region selected when drag was started (region is
    /// assumed to be forward).
    min: usize,

    /// End of the region selected when drag was started.
    max: usize,

    granularity: SelectionGranularity,
}

impl View {
    pub fn new() -> View {
        View {
            selection: SelRegion::caret(0).into(),
            drag_state: None,
            lines: Lines::default(),
        }
    }

    pub(crate) fn do_edit(&mut self, text: &Rope, cmd: ViewEvent) {
        use self::ViewEvent::*;
        match cmd {
            Move(movement) => self.do_move(text, movement, false),
            ModifySelection(movement) => self.do_move(text, movement, true),
            SelectAll => self.select_all(text),
            AddSelectionAbove => self.add_selection_by_movement(text, Movement::UpExactPosition),
            AddSelectionBelow => self.add_selection_by_movement(text, Movement::DownExactPosition),
            Gesture { line, col, ty } => self.do_gesture(text, line, col, ty),
        }
    }

    fn do_gesture(&mut self, text: &Rope, line: u64, col: u64, ty: GestureType) {
        let line = line as usize;
        let col = col as usize;
        let offset = self.line_col_to_offset(text, line, col);
        match ty {
            GestureType::Select { granularity, multi } => {
                self.select(text, offset, granularity, multi)
            }
            GestureType::SelectExtend { granularity } => {
                self.extend_selection(text, offset, granularity)
            }
            GestureType::Drag => self.do_drag(text, offset, Affinity::default()),
        }
    }

    /// Removes any selection present at the given offset.
    /// Returns true if a selection was removed, false otherwise.
    pub fn deselect_at_offset(&mut self, text: &Rope, offset: usize) -> bool {
        if !self.selection.regions_in_range(offset, offset).is_empty() {
            let mut sel = self.selection.clone();
            sel.delete_range(offset, offset, true);
            if !sel.is_empty() {
                self.drag_state = None;
                self.set_selection_raw(text, sel);
                return true;
            }
        }
        false
    }

    /// Move the selection by the given movement. Return value is the offset of
    /// a point that should be scrolled into view.
    ///
    /// If `modify` is `true`, the selections are modified, otherwise the results
    /// of individual region movements become carets.
    pub fn do_move(&mut self, text: &Rope, movement: Movement, modify: bool) {
        self.drag_state = None;
        let new_sel = selection_movement(movement, &self.selection, self, text, modify);
        self.set_selection(text, new_sel);
    }

    /// Set the selection to a new value.
    pub fn set_selection<S: Into<Selection>>(&mut self, text: &Rope, sel: S) {
        self.set_selection_raw(text, sel.into());
    }

    /// Sets the selection to a new value, without invalidating.
    fn set_selection_for_edit(&mut self, text: &Rope, sel: Selection) {
        self.selection = sel;
    }

    /// Sets the selection to a new value, invalidating the line cache as needed.
    /// This function does not perform any scrolling.
    fn set_selection_raw(&mut self, text: &Rope, sel: Selection) {
        self.invalidate_selection(text);
        self.selection = sel;
        self.invalidate_selection(text);
    }

    /// Invalidate the current selection. Note that we could be even more
    /// fine-grained in the case of multiple cursors, but we also want this
    /// method to be fast even when the selection is large.
    fn invalidate_selection(&mut self, text: &Rope) {
        // TODO: refine for upstream (caret appears on prev line)
        let first_line = self.line_of_offset(text, self.selection.first().unwrap().min());
        let last_line = self.line_of_offset(text, self.selection.last().unwrap().max()) + 1;
        let all_caret = self.selection.iter().all(|region| region.is_caret());
    }

    fn add_selection_by_movement(&mut self, text: &Rope, movement: Movement) {
        let mut sel = Selection::new();
        for &region in self.sel_regions() {
            sel.add_region(region);
            let new_region = region_movement(movement, region, self, &text, false);
            sel.add_region(new_region);
        }
        self.set_selection(text, sel);
    }

    // TODO: insert from keyboard or input method shouldn't break undo group,
    /// Invalidates the styles of the given range (start and end are offsets within
    /// the text).
    pub fn invalidate_styles(&mut self, text: &Rope, start: usize, end: usize) {
        let first_line = self.line_of_offset(text, start);
        let (mut last_line, last_col) = self.offset_to_line_col(text, end);
        last_line += if last_col > 0 { 1 } else { 0 };
    }

    /// Select entire buffer.
    ///
    /// Note: unlike movement based selection, this does not scroll.
    pub fn select_all(&mut self, text: &Rope) {
        let selection = SelRegion::new(0, text.len()).into();
        self.set_selection_raw(text, selection);
    }

    /// Finds the unit of text containing the given offset.
    fn unit(&self, text: &Rope, offset: usize, granularity: SelectionGranularity) -> Interval {
        match granularity {
            SelectionGranularity::Point => Interval::new(offset, offset),
            SelectionGranularity::Word => {
                let mut word_cursor = WordCursor::new(text, offset);
                let (start, end) = word_cursor.select_word();
                Interval::new(start, end)
            }
            SelectionGranularity::Line => {
                let (line, _) = self.offset_to_line_col(text, offset);
                let (start, end) = self.lines.logical_line_range(text, line);
                Interval::new(start, end)
            }
        }
    }

    /// Selects text with a certain granularity and supports multi_selection
    fn select(
        &mut self,
        text: &Rope,
        offset: usize,
        granularity: SelectionGranularity,
        multi: bool,
    ) {
        // If multi-select is enabled, toggle existing regions
        if multi
            && granularity == SelectionGranularity::Point
            && self.deselect_at_offset(text, offset)
        {
            return;
        }

        let region = self.unit(text, offset, granularity).into();

        let base_sel = match multi {
            true => self.selection.clone(),
            false => Selection::new(),
        };
        let mut selection = base_sel.clone();
        selection.add_region(region);
        self.set_selection(text, selection);

        self.drag_state = Some(DragState {
            base_sel,
            min: region.start,
            max: region.end,
            granularity,
        });
    }

    /// Extends an existing selection (eg. when the user performs SHIFT + click).
    pub fn extend_selection(
        &mut self,
        text: &Rope,
        offset: usize,
        granularity: SelectionGranularity,
    ) {
        if self.sel_regions().is_empty() {
            return;
        }

        let (base_sel, last) = {
            let mut base = Selection::new();
            let (last, rest) = self.sel_regions().split_last().unwrap();
            for &region in rest {
                base.add_region(region);
            }
            (base, *last)
        };

        let mut sel = base_sel.clone();
        self.drag_state = Some(DragState {
            base_sel,
            min: last.start,
            max: last.start,
            granularity,
        });

        let start = (last.start, last.start);
        let new_region = self.range_region(text, start, offset, granularity);

        // TODO: small nit, merged region should be backward if end < start.
        // This could be done by explicitly overriding, or by tweaking the
        // merge logic.
        sel.add_region(new_region);
        self.set_selection(text, sel);
    }

    /// Splits current selections into lines.
    fn do_split_selection_into_lines(&mut self, text: &Rope) {
        let mut selection = Selection::new();

        for region in self.selection.iter() {
            if region.is_caret() {
                selection.add_region(SelRegion::caret(region.max()));
            } else {
                let mut cursor = Cursor::new(&text, region.min());

                while cursor.pos() < region.max() {
                    let sel_start = cursor.pos();
                    let end_of_line = match cursor.next::<LinesMetric>() {
                        Some(end) if end >= region.max() => max(0, region.max() - 1),
                        Some(end) => max(0, end - 1),
                        None if cursor.pos() == text.len() => cursor.pos(),
                        _ => break,
                    };

                    selection.add_region(SelRegion::new(sel_start, end_of_line));
                }
            }
        }

        self.set_selection_raw(text, selection);
    }

    /// Does a drag gesture, setting the selection from a combination of the drag
    /// state and new offset.
    fn do_drag(&mut self, text: &Rope, offset: usize, affinity: Affinity) {
        let new_sel = self.drag_state.as_ref().map(|drag_state| {
            let mut sel = drag_state.base_sel.clone();
            let start = (drag_state.min, drag_state.max);
            let new_region = self.range_region(text, start, offset, drag_state.granularity);
            sel.add_region(new_region.with_horiz(None).with_affinity(affinity));
            sel
        });

        if let Some(sel) = new_sel {
            self.set_selection(text, sel);
        }
    }

    /// Creates a `SelRegion` for range select or drag operations.
    pub fn range_region(
        &self,
        text: &Rope,
        start: (usize, usize),
        offset: usize,
        granularity: SelectionGranularity,
    ) -> SelRegion {
        let (min_start, max_start) = start;
        let end = self.unit(text, offset, granularity);
        let (min_end, max_end) = (end.start, end.end);
        if offset >= min_start {
            SelRegion::new(min_start, max_end)
        } else {
            SelRegion::new(max_start, min_end)
        }
    }

    /// Returns the regions of the current selection.
    pub fn sel_regions(&self) -> &[SelRegion] {
        &self.selection
    }

    /// Collapse all selections in this view into a single caret
    pub fn collapse_selections(&mut self, text: &Rope) {
        let mut sel = self.selection.clone();
        sel.collapse();
        self.set_selection(text, sel);
    }

    /// Determines whether the offset is in any selection (counting carets and
    /// selection edges).
    pub fn is_point_in_selection(&self, offset: usize) -> bool {
        !self.selection.regions_in_range(offset, offset).is_empty()
    }

    // How should we count "column"? Valid choices include:
    // * Unicode codepoints
    // * grapheme clusters
    // * Unicode width (so CJK counts as 2)
    // * Actual measurement in text layout
    // * Code units in some encoding
    //
    // Of course, all these are identical for ASCII. For now we use UTF-8 code units
    // for simplicity.

    pub(crate) fn offset_to_line_col(&self, text: &Rope, offset: usize) -> (usize, usize) {
        let line = self.line_of_offset(text, offset);
        (line, offset - self.offset_of_line(text, line))
    }

    pub(crate) fn line_col_to_offset(&self, text: &Rope, line: usize, col: usize) -> usize {
        let mut offset = self.offset_of_line(text, line).saturating_add(col);
        if offset >= text.len() {
            offset = text.len();
            if self.line_of_offset(text, offset) <= line {
                return offset;
            }
        } else {
            // Snap to grapheme cluster boundary
            offset = text.prev_grapheme_offset(offset + 1).unwrap();
        }

        // clamp to end of line
        let next_line_offset = self.offset_of_line(text, line + 1);
        if offset >= next_line_offset {
            if let Some(prev) = text.prev_grapheme_offset(next_line_offset) {
                offset = prev;
            }
        }
        offset
    }

    // use own breaks if present, or text if not (no line wrapping)

    /// Returns the visible line number containing the given offset.
    pub fn line_of_offset(&self, text: &Rope, offset: usize) -> usize {
        self.lines.visual_line_of_offset(text, offset)
    }

    /// Returns the byte offset corresponding to the given visual line.
    pub fn offset_of_line(&self, text: &Rope, line: usize) -> usize {
        self.lines.offset_of_visual_line(text, line)
    }

    /// Updates the view after the text has been modified by the given `delta`.
    /// This method is responsible for updating the cursors, and also for
    /// recomputing line wraps.
    pub fn after_edit(
        &mut self,
        text: &Rope,
        last_text: &Rope,
        delta: &RopeDelta,
        drift: InsertDrift,
    ) {
        // Note: for committing plugin edits, we probably want to know the priority
        // of the delta so we can set the cursor before or after the edit, as needed.
        let new_sel = self.selection.apply_delta(delta, true, drift);
        self.set_selection_for_edit(text, new_sel);
    }

    /// Get the line range of a selected region.
    pub fn get_line_range(&self, text: &Rope, region: &SelRegion) -> Range<usize> {
        let (first_line, _) = self.offset_to_line_col(text, region.min());
        let (mut last_line, last_col) = self.offset_to_line_col(text, region.max());
        if last_col == 0 && last_line > first_line {
            last_line -= 1;
        }

        first_line..(last_line + 1)
    }

    pub fn get_caret_offset(&self) -> Option<usize> {
        match self.selection.len() {
            1 if self.selection[0].is_caret() => {
                let offset = self.selection[0].start;
                Some(offset)
            }
            _ => None,
        }
    }
}

// utility function to clamp a value within the given range
fn clamp(x: usize, min: usize, max: usize) -> usize {
    if x < min {
        min
    } else if x < max {
        x
    } else {
        max
    }
}
