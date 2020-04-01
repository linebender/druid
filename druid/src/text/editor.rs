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

use std::borrow::Cow;
use std::collections::BTreeSet;

use xi_rope::rope::count_newlines;
use xi_rope::{Cursor, DeltaBuilder, Interval, LinesMetric, Rope, RopeDelta};

use super::config::BufferItems;
use super::edit_types::BufferEvent;
use super::movement::{region_movement, Movement};
use super::selection::{SelRegion, Selection};
use super::view::View;
use super::word_boundaries::WordCursor;

use super::backspace::offset_for_delete_backwards;

// TODO This could go much higher without issue but while developing it is
// better to keep it low to expose bugs in the GC during casual testing.
const MAX_UNDOS: usize = 20;

enum IndentDirection {
    In,
    Out,
}

pub struct Editor {
    /// The contents of the buffer.
    text: Rope,
}

impl Editor {
    /// Creates a new `Editor` with a new empty buffer.
    pub fn new() -> Editor {
        Self::with_text("")
    }

    /// Creates a new `Editor`, loading text into a new buffer.
    pub fn with_text<T: Into<Rope>>(text: T) -> Editor {
        Editor { text: text.into() }
    }

    pub(crate) fn get_buffer(&self) -> &Rope {
        &self.text
    }

    fn insert<T: Into<Rope>>(&mut self, view: &View, text: T) {
        let rope = text.into();
        let mut builder = DeltaBuilder::new(self.text.len());
        for region in view.sel_regions() {
            let iv = Interval::new(region.min(), region.max());
            builder.replace(iv, rope.clone());
        }
        self.add_delta(builder.build());
    }

    /// Leaves the current selection untouched, but surrounds it with two insertions.
    fn surround<BT, AT>(&mut self, view: &View, before_text: BT, after_text: AT)
    where
        BT: Into<Rope>,
        AT: Into<Rope>,
    {
        let mut builder = DeltaBuilder::new(self.text.len());
        let before_rope = before_text.into();
        let after_rope = after_text.into();
        for region in view.sel_regions() {
            let before_iv = Interval::new(region.min(), region.min());
            builder.replace(before_iv, before_rope.clone());
            let after_iv = Interval::new(region.max(), region.max());
            builder.replace(after_iv, after_rope.clone());
        }
        self.add_delta(builder.build());
    }

    /// Applies a delta to the text, and updates undo state.
    ///
    /// Records the delta into the CRDT engine so that it can be undone. Also
    /// contains the logic for merging edits into the same undo group. At call
    /// time, self.this_edit_type should be set appropriately.
    ///
    /// This method can be called multiple times, accumulating deltas that will
    /// be committed at once with `commit_delta`. Note that it does not update
    /// the views. Thus, view-associated state such as the selection and line
    /// breaks are to be considered invalid after this method, until the
    /// `commit_delta` call.
    fn add_delta(&mut self, delta: RopeDelta) {
        self.text = delta.apply(&self.text);
    }

    fn delete_backward(&mut self, view: &View, config: &BufferItems) {
        // TODO: this function is workable but probably overall code complexity
        // could be improved by implementing a "backspace" movement instead.
        let mut builder = DeltaBuilder::new(self.text.len());
        for region in view.sel_regions() {
            let start = offset_for_delete_backwards(&view, &region, &self.text, &config);
            let iv = Interval::new(start, region.max());
            if !iv.is_empty() {
                builder.delete(iv);
            }
        }

        if !builder.is_empty() {
            self.add_delta(builder.build());
        }
    }

    /// Common logic for a number of delete methods. For each region in the
    /// selection, if the selection is a caret, delete the region between
    /// the caret and the movement applied to the caret, otherwise delete
    /// the region.
    ///
    /// If `save` is set, save the deleted text into the kill ring.
    fn delete_by_movement(
        &mut self,
        view: &View,
        movement: Movement,
        save: bool,
        kill_ring: &mut Rope,
    ) {
        // We compute deletions as a selection because the merge logic
        // is convenient. Another possibility would be to make the delta
        // builder able to handle overlapping deletions (with union semantics).
        let mut deletions = Selection::new();
        for &r in view.sel_regions() {
            if r.is_caret() {
                let new_region = region_movement(movement, r, view, &self.text, true);
                deletions.add_region(new_region);
            } else {
                deletions.add_region(r);
            }
        }
        if save {
            let saved = self.extract_sel_regions(&deletions).unwrap_or_default();
            *kill_ring = Rope::from(saved);
        }
        self.delete_sel_regions(&deletions);
    }

    /// Deletes the given regions.
    fn delete_sel_regions(&mut self, sel_regions: &[SelRegion]) {
        let mut builder = DeltaBuilder::new(self.text.len());
        for region in sel_regions {
            let iv = Interval::new(region.min(), region.max());
            if !iv.is_empty() {
                builder.delete(iv);
            }
        }
        if !builder.is_empty() {
            self.add_delta(builder.build());
        }
    }

    /// Extracts non-caret selection regions into a string,
    /// joining multiple regions with newlines.
    fn extract_sel_regions(&self, sel_regions: &[SelRegion]) -> Option<Cow<str>> {
        let mut saved = None;
        for region in sel_regions {
            if !region.is_caret() {
                let val = self.text.slice_to_cow(region);
                match saved {
                    None => saved = Some(val),
                    Some(ref mut s) => {
                        s.to_mut().push('\n');
                        s.to_mut().push_str(&val);
                    }
                }
            }
        }
        saved
    }

    fn insert_newline(&mut self, view: &View, config: &BufferItems) {
        self.insert(view, &config.line_ending);
    }

    fn insert_tab(&mut self, view: &View, config: &BufferItems) {
        let mut builder = DeltaBuilder::new(self.text.len());
        let const_tab_text = self.get_tab_text(config, None);

        for region in view.sel_regions() {
            let line_range = view.get_line_range(&self.text, region);

            if line_range.len() > 1 {
                for line in line_range {
                    let offset = view.line_col_to_offset(&self.text, line, 0);
                    let iv = Interval::new(offset, offset);
                    builder.replace(iv, Rope::from(const_tab_text));
                }
            } else {
                let (_, col) = view.offset_to_line_col(&self.text, region.start);
                let mut tab_size = config.tab_size;
                tab_size = tab_size - (col % tab_size);
                let tab_text = self.get_tab_text(config, Some(tab_size));

                let iv = Interval::new(region.min(), region.max());
                builder.replace(iv, Rope::from(tab_text));
            }
        }
        self.add_delta(builder.build());
    }

    /// Indents or outdents lines based on selection and user's tab settings.
    /// Uses a BTreeSet to holds the collection of lines to modify.
    /// Preserves cursor position and current selection as much as possible.
    /// Tries to have behavior consistent with other editors like Atom,
    /// Sublime and VSCode, with non-caret selections not being modified.
    fn modify_indent(&mut self, view: &View, config: &BufferItems, direction: IndentDirection) {
        let mut lines = BTreeSet::new();
        let tab_text = self.get_tab_text(config, None);
        for region in view.sel_regions() {
            let line_range = view.get_line_range(&self.text, region);
            for line in line_range {
                lines.insert(line);
            }
        }
        match direction {
            IndentDirection::In => self.indent(view, lines, tab_text),
            IndentDirection::Out => self.outdent(view, lines, tab_text),
        };
    }

    fn indent(&mut self, view: &View, lines: BTreeSet<usize>, tab_text: &str) {
        let mut builder = DeltaBuilder::new(self.text.len());
        for line in lines {
            let offset = view.line_col_to_offset(&self.text, line, 0);
            let interval = Interval::new(offset, offset);
            builder.replace(interval, Rope::from(tab_text));
        }
        self.add_delta(builder.build());
    }

    fn outdent(&mut self, view: &View, lines: BTreeSet<usize>, tab_text: &str) {
        let mut builder = DeltaBuilder::new(self.text.len());
        for line in lines {
            let offset = view.line_col_to_offset(&self.text, line, 0);
            let tab_offset = view.line_col_to_offset(&self.text, line, tab_text.len());
            let interval = Interval::new(offset, tab_offset);
            let leading_slice = self.text.slice_to_cow(interval.start()..interval.end());
            if leading_slice == tab_text {
                builder.delete(interval);
            } else if let Some(first_char_col) = leading_slice.find(|c: char| !c.is_whitespace()) {
                let first_char_offset = view.line_col_to_offset(&self.text, line, first_char_col);
                let interval = Interval::new(offset, first_char_offset);
                builder.delete(interval);
            }
        }
        self.add_delta(builder.build());
    }

    fn get_tab_text(&self, config: &BufferItems, tab_size: Option<usize>) -> &'static str {
        let tab_size = tab_size.unwrap_or(config.tab_size);
        let tab_text = if config.translate_tabs_to_spaces {
            n_spaces(tab_size)
        } else {
            "\t"
        };

        tab_text
    }

    fn do_insert(&mut self, view: &View, config: &BufferItems, chars: &str) {
        let pair_search = config.surrounding_pairs.iter().find(|pair| pair.0 == chars);
        let caret_exists = view.sel_regions().iter().any(|region| region.is_caret());
        if let (Some(pair), false) = (pair_search, caret_exists) {
            self.surround(view, pair.0.to_string(), pair.1.to_string());
        } else {
            self.insert(view, chars);
        }
    }

    fn do_paste(&mut self, view: &View, chars: &str) {
        if view.sel_regions().len() == 1 || view.sel_regions().len() != count_lines(chars) {
            self.insert(view, chars);
        } else {
            let mut builder = DeltaBuilder::new(self.text.len());
            for (sel, line) in view.sel_regions().iter().zip(chars.lines()) {
                let iv = Interval::new(sel.min(), sel.max());
                builder.replace(iv, line.into());
            }
            self.add_delta(builder.build());
        }
    }

    fn sel_region_to_interval_and_rope(&self, region: SelRegion) -> (Interval, Rope) {
        let as_interval = Interval::new(region.min(), region.max());
        let interval_rope = self.text.subseq(as_interval);
        (as_interval, interval_rope)
    }

    fn do_transpose(&mut self, view: &View) {
        let mut builder = DeltaBuilder::new(self.text.len());
        let mut last = 0;
        let mut optional_previous_selection: Option<(Interval, Rope)> =
            last_selection_region(view.sel_regions())
                .map(|&region| self.sel_region_to_interval_and_rope(region));

        for &region in view.sel_regions() {
            if region.is_caret() {
                let mut middle = region.end;
                let mut start = self.text.prev_grapheme_offset(middle).unwrap_or(0);
                let mut end = self.text.next_grapheme_offset(middle).unwrap_or(middle);

                // Note: this matches Emac's behavior. It swaps last
                // two characters of line if at end of line.
                if start >= last {
                    let end_line_offset =
                        view.offset_of_line(&self.text, view.line_of_offset(&self.text, end));
                    // include end != self.text.len() because if the editor is entirely empty, we dont' want to pull from empty space
                    if (end == middle || end == end_line_offset) && end != self.text.len() {
                        middle = start;
                        start = self.text.prev_grapheme_offset(middle).unwrap_or(0);
                        end = middle.wrapping_add(1);
                    }

                    let interval = Interval::new(start, end);
                    let before = self.text.slice_to_cow(start..middle);
                    let after = self.text.slice_to_cow(middle..end);
                    let swapped: String = [after, before].concat();
                    builder.replace(interval, Rope::from(swapped));
                    last = end;
                }
            } else if let Some(previous_selection) = optional_previous_selection {
                let current_interval = self.sel_region_to_interval_and_rope(region);
                builder.replace(current_interval.0, previous_selection.1);
                optional_previous_selection = Some(current_interval);
            }
        }
        if !builder.is_empty() {
            self.add_delta(builder.build());
        }
    }

    fn yank(&mut self, view: &View, kill_ring: &mut Rope) {
        // TODO: if there are multiple cursors and the number of newlines
        // is one less than the number of cursors, split and distribute one
        // line per cursor.
        self.insert(view, kill_ring.clone());
    }

    fn transform_text<F: Fn(&str) -> String>(&mut self, view: &View, transform_function: F) {
        let mut builder = DeltaBuilder::new(self.text.len());

        for region in view.sel_regions() {
            let selected_text = self.text.slice_to_cow(region);
            let interval = Interval::new(region.min(), region.max());
            builder.replace(interval, Rope::from(transform_function(&selected_text)));
        }
        if !builder.is_empty() {
            self.add_delta(builder.build());
        }
    }

    /// Changes the number(s) under the cursor(s) with the `transform_function`.
    /// If there is a number next to or on the beginning of the region, then
    /// this number will be replaced with the result of `transform_function` and
    /// the cursor will be placed at the end of the number.
    /// Some Examples with a increment `transform_function`:
    ///
    /// "|1234" -> "1235|"
    /// "12|34" -> "1235|"
    /// "-|12" -> "-11|"
    /// "another number is 123|]" -> "another number is 124"
    ///
    /// This function also works fine with multiple regions.
    fn change_number<F: Fn(i128) -> Option<i128>>(&mut self, view: &View, transform_function: F) {
        let mut builder = DeltaBuilder::new(self.text.len());
        for region in view.sel_regions() {
            let mut cursor = WordCursor::new(&self.text, region.end);
            let (mut start, end) = cursor.select_word();

            // if the word begins with '-', then it is a negative number
            if start > 0 && self.text.byte_at(start - 1) == (b'-') {
                start -= 1;
            }

            let word = self.text.slice_to_cow(start..end);
            if let Some(number) = word.parse::<i128>().ok().and_then(&transform_function) {
                let interval = Interval::new(start, end);
                builder.replace(interval, Rope::from(number.to_string()));
            }
        }

        if !builder.is_empty() {
            self.add_delta(builder.build());
        }
    }

    // capitalization behaviour is similar to behaviour in XCode
    fn capitalize_text(&mut self, view: &mut View) {
        let mut builder = DeltaBuilder::new(self.text.len());
        let mut final_selection = Selection::new();

        for &region in view.sel_regions() {
            final_selection.add_region(SelRegion::new(region.max(), region.max()));
            let mut word_cursor = WordCursor::new(&self.text, region.min());

            loop {
                // capitalize each word in the current selection
                let (start, end) = word_cursor.select_word();

                if start < end {
                    let interval = Interval::new(start, end);
                    let word = self.text.slice_to_cow(start..end);

                    // first letter is uppercase, remaining letters are lowercase
                    let (first_char, rest) = word.split_at(1);
                    let capitalized_text =
                        [first_char.to_uppercase(), rest.to_lowercase()].concat();
                    builder.replace(interval, Rope::from(capitalized_text));
                }

                if word_cursor.next_boundary().is_none() || end > region.max() {
                    break;
                }
            }
        }

        if !builder.is_empty() {
            self.add_delta(builder.build());
        }

        // at the end of the transformation carets are located at the end of the words that were
        // transformed last in the selections
        view.collapse_selections(&self.text);
        view.set_selection(&self.text, final_selection);
    }

    fn duplicate_line(&mut self, view: &View, config: &BufferItems) {
        let mut builder = DeltaBuilder::new(self.text.len());
        // get affected lines or regions
        let mut to_duplicate = BTreeSet::new();

        for region in view.sel_regions() {
            let (first_line, _) = view.offset_to_line_col(&self.text, region.min());
            let line_start = view.offset_of_line(&self.text, first_line);

            let mut cursor = match region.is_caret() {
                true => Cursor::new(&self.text, line_start),
                false => {
                    // duplicate all lines together that are part of the same selections
                    let (last_line, _) = view.offset_to_line_col(&self.text, region.max());
                    let line_end = view.offset_of_line(&self.text, last_line);
                    Cursor::new(&self.text, line_end)
                }
            };

            if let Some(line_end) = cursor.next::<LinesMetric>() {
                to_duplicate.insert((line_start, line_end));
            }
        }

        for (start, end) in to_duplicate {
            // insert duplicates
            let iv = Interval::new(start, start);
            builder.replace(iv, self.text.slice(start..end));

            // last line does not have new line character so it needs to be manually added
            if end == self.text.len() {
                builder.replace(iv, Rope::from(&config.line_ending))
            }
        }

        self.add_delta(builder.build());
    }

    pub(crate) fn do_edit(
        &mut self,
        view: &mut View,
        kill_ring: &mut Rope,
        config: &BufferItems,
        cmd: BufferEvent,
    ) {
        use self::BufferEvent::*;
        match cmd {
            Delete { movement, kill } => self.delete_by_movement(view, movement, kill, kill_ring),
            Backspace => self.delete_backward(view, config),
            Transpose => self.do_transpose(view),
            Undo => unimplemented!(),
            Redo => unimplemented!(),
            Uppercase => self.transform_text(view, |s| s.to_uppercase()),
            Lowercase => self.transform_text(view, |s| s.to_lowercase()),
            Capitalize => self.capitalize_text(view),
            Indent => self.modify_indent(view, config, IndentDirection::In),
            Outdent => self.modify_indent(view, config, IndentDirection::Out),
            InsertNewline => self.insert_newline(view, config),
            InsertTab => self.insert_tab(view, config),
            Insert(chars) => self.do_insert(view, config, &chars),
            Paste(chars) => self.do_paste(view, &chars),
            Yank => self.yank(view, kill_ring),
            DuplicateLine => self.duplicate_line(view, config),
            IncreaseNumber => self.change_number(view, |s| s.checked_add(1)),
            DecreaseNumber => self.change_number(view, |s| s.checked_sub(1)),
        }
    }
}

fn last_selection_region(regions: &[SelRegion]) -> Option<&SelRegion> {
    for region in regions.iter().rev() {
        if !region.is_caret() {
            return Some(region);
        }
    }
    None
}

fn n_spaces(n: usize) -> &'static str {
    let spaces = "                                ";
    assert!(n <= spaces.len());
    &spaces[..n]
}

/// Counts the number of lines in the string, not including any trailing newline.
fn count_lines(s: &str) -> usize {
    let mut newlines = count_newlines(s);
    if s.as_bytes().last() == Some(&0xa) {
        newlines -= 1;
    }
    1 + newlines
}
