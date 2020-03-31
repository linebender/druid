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

//! Compute line wrapping breaks for text.

use std::cmp::Ordering;

use xi_rope::breaks::{Breaks, BreaksInfo, BreaksMetric};
use xi_rope::{Cursor, Interval, LinesMetric, Rope, RopeInfo};
use xi_unicode::LineBreakLeafIter;

/// A range to be rewrapped.
type Task = Interval;

/// Tracks state related to visual lines.
#[derive(Default)]
pub(crate) struct Lines {
    breaks: Breaks,
    /// Aka the 'frontier'; ranges of lines that still need to be wrapped.
    work: Vec<Task>,
}

pub(crate) struct VisualLine {
    pub(crate) interval: Interval,
    /// The logical line number for this line. Only present when this is the
    /// first visual line in a logical line.
    pub(crate) line_num: Option<usize>,
}

impl VisualLine {
    fn new<I: Into<Interval>, L: Into<Option<usize>>>(iv: I, line: L) -> Self {
        VisualLine {
            interval: iv.into(),
            line_num: line.into(),
        }
    }
}

/// Describes what has changed after a batch of word wrapping; this is used
/// for minimal invalidation.
pub(crate) struct InvalLines {
    pub(crate) start_line: usize,
    pub(crate) inval_count: usize,
    pub(crate) new_count: usize,
}

/// Detailed information about changes to linebreaks, used to generate
/// invalidation information. The logic behind invalidation is different
/// depending on whether we're updating breaks after an edit, or continuing
/// a bulk rewrapping task. This is shared between the two cases, and contains
/// the information relevant to both of them.
struct WrapSummary {
    start_line: usize,
    /// Total number of invalidated lines; this is meaningless in the after_edit case.
    inval_count: usize,
    /// The total number of new (hard + soft) breaks in the wrapped region.
    new_count: usize,
    /// The number of new soft breaks.
    new_soft: usize,
}

impl Lines {
    pub(crate) fn visual_line_of_offset(&self, text: &Rope, offset: usize) -> usize {
        let mut line = text.line_of_offset(offset);
        line
    }

    /// Returns the byte offset corresponding to the line `line`.
    pub(crate) fn offset_of_visual_line(&self, text: &Rope, line: usize) -> usize {
        // sanitize input
        let line = line.min(text.measure::<LinesMetric>() + 1);
        text.offset_of_line(line)
    }

    /// Returns an iterator over [`VisualLine`]s, starting at (and including)
    /// `start_line`.
    pub(crate) fn iter_lines<'a>(
        &'a self,
        text: &'a Rope,
        start_line: usize,
    ) -> impl Iterator<Item = VisualLine> + 'a {
        let mut cursor = MergedBreaks::new(text, &self.breaks);
        let offset = cursor.offset_of_line(start_line);
        let logical_line = text.line_of_offset(offset) + 1;
        cursor.set_offset(offset);
        VisualLines {
            offset,
            cursor,
            len: text.len(),
            logical_line,
            eof: false,
        }
    }

    /// Adjust offsets for any tasks after an edit.
    fn patchup_tasks<T: Into<Interval>>(&mut self, iv: T, new_len: usize) {
        let iv = iv.into();
        let mut new_work = Vec::new();

        for task in &self.work {
            if task.is_before(iv.start) {
                new_work.push(*task);
            } else if task.contains(iv.start) {
                let head = task.prefix(iv);
                let tail_end = iv.start.max((task.end + new_len).saturating_sub(iv.size()));
                let tail = Interval::new(iv.start, tail_end);
                new_work.push(head);
                new_work.push(tail);
            } else {
                // take task - our edit interval, then translate it (- old_size, + new_size)
                let tail = task.suffix(iv).translate(new_len).translate_neg(iv.size());
                new_work.push(tail);
            }
        }
        new_work.retain(|iv| !iv.is_empty());
        self.work.clear();
        for task in new_work {
            if let Some(prev) = self.work.last_mut() {
                if prev.end >= task.start {
                    *prev = prev.union(task);
                    continue;
                }
            }
            self.work.push(task);
        }
    }

    pub fn logical_line_range(&self, text: &Rope, line: usize) -> (usize, usize) {
        let mut cursor = MergedBreaks::new(text, &self.breaks);
        let offset = cursor.offset_of_line(line);
        let logical_line = text.line_of_offset(offset);
        let start_logical_line_offset = text.offset_of_line(logical_line);
        let end_logical_line_offset = text.offset_of_line(logical_line + 1);
        (start_logical_line_offset, end_logical_line_offset)
    }
}

struct LineBreakCursor<'a> {
    inner: Cursor<'a, RopeInfo>,
    lb_iter: LineBreakLeafIter,
    last_byte: u8,
}

impl<'a> LineBreakCursor<'a> {
    fn new(text: &'a Rope, pos: usize) -> LineBreakCursor<'a> {
        let inner = Cursor::new(text, pos);
        let lb_iter = match inner.get_leaf() {
            Some((s, offset)) => LineBreakLeafIter::new(s.as_str(), offset),
            _ => LineBreakLeafIter::default(),
        };
        LineBreakCursor {
            inner,
            lb_iter,
            last_byte: 0,
        }
    }

    // position and whether break is hard; up to caller to stop calling after EOT
    fn next(&mut self) -> (usize, bool) {
        let mut leaf = self.inner.get_leaf();
        loop {
            match leaf {
                Some((s, offset)) => {
                    let (next, hard) = self.lb_iter.next(s.as_str());
                    if next < s.len() {
                        return (self.inner.pos() - offset + next, hard);
                    }
                    if !s.is_empty() {
                        self.last_byte = s.as_bytes()[s.len() - 1];
                    }
                    leaf = self.inner.next_leaf();
                }
                // A little hacky but only reports last break as hard if final newline
                None => return (self.inner.pos(), self.last_byte == b'\n'),
            }
        }
    }
}

struct VisualLines<'a> {
    cursor: MergedBreaks<'a>,
    offset: usize,
    /// The current logical line number.
    logical_line: usize,
    len: usize,
    eof: bool,
}

impl<'a> Iterator for VisualLines<'a> {
    type Item = VisualLine;

    fn next(&mut self) -> Option<VisualLine> {
        let line_num = if self.cursor.is_hard_break() {
            Some(self.logical_line)
        } else {
            None
        };
        let next_end_bound = match self.cursor.next() {
            Some(b) => b,
            None if self.eof => return None,
            _else => {
                self.eof = true;
                self.len
            }
        };
        let result = VisualLine::new(self.offset..next_end_bound, line_num);
        if self.cursor.is_hard_break() {
            self.logical_line += 1;
        }
        self.offset = next_end_bound;
        Some(result)
    }
}

/// A cursor over both hard and soft breaks. Hard breaks are retrieved from
/// the rope; the soft breaks are stored independently; this interleaves them.
///
/// # Invariants:
///
/// `self.offset` is always a valid break in one of the cursors, unless
/// at 0 or EOF.
///
/// `self.offset == self.text.pos().min(self.soft.pos())`.
struct MergedBreaks<'a> {
    text: Cursor<'a, RopeInfo>,
    soft: Cursor<'a, BreaksInfo>,
    offset: usize,
    /// Starting from zero, how many calls to `next` to get to `self.offset`?
    cur_line: usize,
    total_lines: usize,
    /// Total length, in base units
    len: usize,
}

impl<'a> Iterator for MergedBreaks<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.text.pos() == self.offset && !self.at_eof() {
            // don't iterate past EOF, or we can't get the leaf and check for \n
            self.text.next::<LinesMetric>();
        }
        if self.soft.pos() == self.offset {
            self.soft.next::<BreaksMetric>();
        }
        let prev_off = self.offset;
        self.offset = self.text.pos().min(self.soft.pos());

        let eof_without_newline = self.offset > 0 && self.at_eof() && self.eof_without_newline();
        if self.offset == prev_off || eof_without_newline {
            None
        } else {
            self.cur_line += 1;
            Some(self.offset)
        }
    }
}

// arrived at this by just trying out a bunch of values ¯\_(ツ)_/¯
/// how far away a line can be before we switch to a binary search
const MAX_LINEAR_DIST: usize = 20;

impl<'a> MergedBreaks<'a> {
    fn new(text: &'a Rope, breaks: &'a Breaks) -> Self {
        debug_assert_eq!(text.len(), breaks.len());
        let text = Cursor::new(text, 0);
        let soft = Cursor::new(breaks, 0);
        let total_lines =
            text.root().measure::<LinesMetric>() + soft.root().measure::<BreaksMetric>() + 1;
        let len = text.total_len();
        MergedBreaks {
            text,
            soft,
            offset: 0,
            cur_line: 0,
            total_lines,
            len,
        }
    }

    /// Sets the `self.offset` to the first valid break immediately at or preceding `offset`,
    /// and restores invariants.
    fn set_offset(&mut self, offset: usize) {
        self.text.set(offset);
        self.soft.set(offset);
        if offset > 0 {
            if self.text.at_or_prev::<LinesMetric>().is_none() {
                self.text.set(0);
            }
            if self.soft.at_or_prev::<BreaksMetric>().is_none() {
                self.soft.set(0);
            }
        }

        // self.offset should be at the first valid break immediately preceding `offset`, or 0.
        // the position of the non-break cursor should be > than that of the break cursor, or EOF.
        match self.text.pos().cmp(&self.soft.pos()) {
            Ordering::Less => {
                self.text.next::<LinesMetric>();
            }
            Ordering::Greater => {
                self.soft.next::<BreaksMetric>();
            }
            Ordering::Equal => assert!(self.text.pos() == 0),
        }

        self.offset = self.text.pos().min(self.soft.pos());
        self.cur_line = merged_line_of_offset(self.text.root(), self.soft.root(), self.offset);
    }

    fn offset_of_line(&mut self, line: usize) -> usize {
        match line {
            0 => 0,
            l if l >= self.total_lines => self.text.total_len(),
            l if l == self.cur_line => self.offset,
            l if l > self.cur_line && l - self.cur_line < MAX_LINEAR_DIST => {
                self.offset_of_line_linear(l)
            }
            other => self.offset_of_line_bsearch(other),
        }
    }

    fn offset_of_line_linear(&mut self, line: usize) -> usize {
        assert!(line > self.cur_line);
        let dist = line - self.cur_line;
        self.nth(dist - 1).unwrap_or(self.len)
    }

    fn offset_of_line_bsearch(&mut self, line: usize) -> usize {
        let mut range = 0..self.len;
        loop {
            let pivot = range.start + (range.end - range.start) / 2;
            self.set_offset(pivot);

            match self.cur_line {
                l if l == line => break self.offset,
                l if l > line => range = range.start..pivot,
                l if line - l > MAX_LINEAR_DIST => range = pivot..range.end,
                _else => break self.offset_of_line_linear(line),
            }
        }
    }

    fn is_hard_break(&self) -> bool {
        self.offset == self.text.pos()
    }

    fn at_eof(&self) -> bool {
        self.offset == self.len
    }

    fn eof_without_newline(&mut self) -> bool {
        debug_assert!(self.at_eof());
        self.text.set(self.len);
        self.text
            .get_leaf()
            .map(|(l, _)| l.as_bytes().last() != Some(&b'\n'))
            .unwrap()
    }
}

fn merged_line_of_offset(text: &Rope, soft: &Breaks, offset: usize) -> usize {
    text.count::<LinesMetric>(offset) + soft.count::<BreaksMetric>(offset)
}
