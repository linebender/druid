// Copyright 2020 The Druid Authors.
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

use crate::common_util::Counter;
use crate::keyboard::{KbKey, KeyEvent};
use crate::kurbo::{Point, Rect};
use crate::piet::HitTestPoint;
use crate::window::WinHandler;
use std::borrow::Cow;
use std::ops::Range;

/// Uniquely identifies a text input field inside a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct TextInputToken(u64);

impl TextInputToken {
    /// A token that does not correspond to any text input.
    pub const INVALID: TextInputToken = TextInputToken(0);

    /// Create a new token; this should for the most part be called only by platform code.
    pub fn next() -> TextInputToken {
        static TEXT_FIELD_COUNTER: Counter = Counter::new();
        TextInputToken(TEXT_FIELD_COUNTER.next())
    }

    /// Create a new token from a raw value.
    pub const fn from_raw(id: u64) -> TextInputToken {
        TextInputToken(id)
    }

    /// Get the raw value for a token.
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

/// An event that indicates to the platform that some aspect of a text field may have changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextInputUpdate {
    /// Indicates the value returned by `TextInputHandler::selection` may have changed.
    SelectionChanged,

    /// Indicates the values returned by one or more of these methods may have changed:
    /// - `TextInputHandler::hit_test_point`
    /// - `TextInputHandler::line_range`
    /// - `TextInputHandler::bounding_box`
    /// - `TextInputHandler::slice_bounding_box`
    LayoutChanged,

    /// Indicates any value returned from any of the TextInputHandler methods may have changed.
    Reset,
}

/// Corresponds to a selection of some text. This also can represent a text input caret — the blinking vertical
/// bar where text is to be inserted. A caret is a selection with zero length, where `anchor == extent`.
/// Indices are always expressed in UTF-8 bytes, and must be between 0 and the document length, inclusive.
///
/// As an example, if the input caret is at the start of the document `hello world`, we would expect both `anchor`
/// and `extent` to be `0`. If the user holds shift and presses the right arrow key five times, we would expect the
/// word `hello` to be selected, the `anchor` to still be `0`, and the `extent` to now be `5`.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct TextSelection {
    /// This is the end of the selection that stays unchanged while holding shift and pressing the arrow keys.
    pub anchor: usize,
    /// This is the end of the selection that moves while holding shift and pressing the arrow keys.
    pub extent: usize,
}

#[allow(clippy::len_without_is_empty)]
impl TextSelection {
    /// Creates a new caret (the blinking vertical bar where text is to be inserted) at UTF-8 byte index `index`.
    /// `index` must be a grapheme cluster boundary.
    pub fn new_caret(index: usize) -> TextSelection {
        TextSelection {
            anchor: index,
            extent: index,
        }
    }

    /// Corresponds to the end of the selection — either the `anchor` or `extent` — that has a lesser byte index.
    /// Because of bidirectional text, this is not necessarily "left".
    pub fn upstream_index(&self) -> usize {
        usize::min(self.anchor, self.extent)
    }

    /// Corresponds to the end of the selection — either the `anchor` or `extent` — that has a greater byte index.
    /// Because of bidirectional text, this is not necessarily "right".
    pub fn downstream_index(&self) -> usize {
        usize::max(self.anchor, self.extent)
    }

    /// The sorted range of the document that would be replaced if text were inserted at this selection.
    pub fn to_range(&self) -> Range<usize> {
        self.upstream_index()..self.downstream_index()
    }

    /// The number of characters in the selection. If the selection is a caret, this is `0`.
    pub fn len(&self) -> usize {
        if self.anchor > self.extent {
            self.anchor - self.extent
        } else {
            self.extent - self.anchor
        }
    }

    /// Returns true if the selection's length is `0`.
    pub fn is_caret(&self) -> bool {
        self.len() == 0
    }
}

/// A lock on a text document, such as a text box or text editor document.
/// The platform acquires this lock temporarily to apply edits corresponding to some user input.
/// So long as the `TextInputHandler` has not been dropped,
/// the only changes to the document state must come from calls to `TextInputHandler`.
///
/// Some methods require a mutable lock, as indicated when acquiring the lock with `WinHandler::text_input`.
/// If a mutable method is called on a immutable lock, `TextInputHandler` may panic.
///
/// All ranges, lengths, and indices are specified in UTF-8 code units, unless specified otherwise.
pub trait TextInputHandler {
    /// Gets the range of the document that is currently selected.
    /// If the selection is a vertical caret bar, then `range.start == range.end`.
    /// Both `selection.anchor` and `selection.extent` must be less than or equal to the value returned
    /// from `TextInputHandler::len()`, and must land on a extended grapheme cluster boundary in the document.
    fn selection(&mut self) -> TextSelection;

    /// Sets the range of the document that is currently selected.
    /// If the selection is a vertical caret bar, then `range.start == range.end`.
    /// Both `selection.anchor` and `selection.extent` must be less than or equal to the value returned
    /// from `TextInputHandler::len()`.
    ///
    /// The `set_selection` implementation should round up (downstream) both `selection.anchor` and `selection.extent`
    /// to the nearest extended grapheme cluster boundary.
    ///
    /// Requries a mutable lock.
    fn set_selection(&mut self, selection: TextSelection);

    /// Gets the range of the document that is the input method's composition region.
    /// Both `range.start` and `range.end` must be less than or equal to the value returned
    /// from `TextInputHandler::len()`, and must land on a extended grapheme cluster boundary in the document.
    fn composition_range(&mut self) -> Option<Range<usize>>;

    /// Sets the range of the document that is the input method's composition region.
    /// Both `range.start` and `range.end` must be less than or equal to the value returned
    /// from `TextInputHandler::len()`.
    ///
    /// The `set_selection` implementation should round up (downstream) both `range.start` and `range.end`
    /// to the nearest extended grapheme cluster boundary.
    ///
    /// Requries a mutable lock.
    fn set_composition_range(&mut self, range: Option<Range<usize>>);

    /// Returns true if `i==0`, `i==TextInputHandler::len()`, or `i` is the first byte of a UTF-8 code point sequence.
    /// Returns false otherwise, including if `i>TextInputHandler::len()`.
    /// Equivalent in functionality to `String::is_char_boundary`.
    fn is_char_boundary(&mut self, i: usize) -> bool;

    /// Returns the length of the document in UTF-8 code units. Equivalent to `String::len`.
    fn len(&mut self) -> usize;

    /// Returns whether the length of the document is `0`.
    fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// Returns the contents of some range of the document.
    /// If `range.start` or `range.end` do not fall on a code point sequence boundary, this method may panic.
    fn slice<'a>(&'a mut self, range: Range<usize>) -> Cow<'a, str>;

    /// Converts the document into UTF-8, looks up the range specified by `utf8_range` (in UTF-8 code units), reencodes
    /// that substring into UTF-16, and then returns the number of UTF-16 code units in that substring.
    ///
    /// This is automatically implemented, but you can override this if you have some faster system to determine string length.
    fn utf8_to_utf16(&mut self, utf8_range: Range<usize>) -> usize {
        self.slice(utf8_range).encode_utf16().count()
    }

    /// Converts the document into UTF-16, looks up the range specified by `utf16_range` (in UTF-16 code units), reencodes
    /// that substring into UTF-8, and then returns the number of UTF-8 code units in that substring.
    ///
    /// This is automatically implemented, but you can override this if you have some faster system to determine string length.
    fn utf16_to_utf8(&mut self, utf16_range: Range<usize>) -> usize {
        if utf16_range.is_empty() {
            return 0;
        }
        let doc_range = 0..self.len();
        let text = self.slice(doc_range);
        let utf16: Vec<u16> = text
            .encode_utf16()
            .skip(utf16_range.start)
            .take(utf16_range.end)
            .collect();
        String::from_utf16_lossy(&utf16).len()
    }

    /// Replaces a range of the text document with `text`.
    /// If `range.start` or `range.end` do not fall on a code point sequence boundary, this method may panic.
    /// Equivalent to `String::replace_range`.
    ///
    /// This method also sets the composition range to `None`, and updates the selection:
    ///
    /// - If both the selection's anchor and extent are `< range.start`, then nothing is updated.
    /// - If both the selection's anchor and extent are `> range.end`, then subtract `range.len()` from both, and add `text.len()`.
    /// - If neither of the previous two conditions are true, then set both anchor and extent to `range.start + text.len()`.
    ///
    /// After the above update, if we increase each end of the selection if necessary to put it on a grapheme cluster boundary.
    ///
    /// Requries a mutable lock.
    fn replace_range(&mut self, range: Range<usize>, text: &str);

    /// Given a `Point`, determine the corresponding text position.
    fn hit_test_point(&mut self, point: Point) -> HitTestPoint;

    /// Returns the character range of the line (soft- or hard-wrapped) containing the character
    /// specified by `char_index`.
    fn line_range(&mut self, char_index: usize, affinity: Affinity) -> Range<usize>;

    /// Returns the bounding box, in window coordinates, of the visible text document. For instance,
    /// a text box's bounding box would be the rectangle of the border surrounding it, even if the text box is empty.
    /// If the text document is completely offscreen, return `None`.
    fn bounding_box(&mut self) -> Option<Rect>;

    /// Returns the bounding box, in window coordinates, of the range of text specified by `range`.
    /// Ranges will always be equal to or a subrange of some line range returned by `TextInputHandler::line_range`.
    /// If a range spans multiple lines, `slice_bounding_box` may panic.
    fn slice_bounding_box(&mut self, range: Range<usize>) -> Option<Rect>;

    /// Applies some special action to the text field.
    ///
    /// Requries a mutable lock.
    fn handle_action(&mut self, action: TextInputAction);
}

#[allow(dead_code)]
/// Simulates `TextInputHandler` calls on `handler` for a given keypress `event`. This circumvents the platform, and so can't
/// work with important features like input method editors! However, it's necessary while we build up our input support on
/// various platforms, which takes a lot of time, and allows applications to start building on the new `TextInputHandler`
/// interface immediately on all platforms, with a hopefully seamless upgrade process as we implement IME input on more platforms.
pub fn simulate_text_input<H: WinHandler + ?Sized>(
    handler: &mut H,
    token: Option<TextInputToken>,
    event: KeyEvent,
) -> bool {
    if handler.key_down(event.clone()) {
        return true;
    }

    let token = match token {
        Some(v) => v,
        None => return false,
    };
    let mut input_handler = handler.text_input(token, true);
    match event.key {
        KbKey::Character(c) if !event.mods.ctrl() && !event.mods.meta() && !event.mods.alt() => {
            let selection = input_handler.selection();
            input_handler.replace_range(selection.to_range(), &c);
            let new_caret_index = selection.upstream_index() + c.len();
            input_handler.set_selection(TextSelection::new_caret(new_caret_index));
        }
        KbKey::ArrowLeft => {
            let movement = TextMovement::Grapheme(TextDirection::Left);
            if event.mods.shift() {
                input_handler.handle_action(TextInputAction::MoveExtent(movement));
            } else {
                input_handler.handle_action(TextInputAction::Move(movement));
            }
        }
        KbKey::ArrowRight => {
            let movement = TextMovement::Grapheme(TextDirection::Right);
            if event.mods.shift() {
                input_handler.handle_action(TextInputAction::MoveExtent(movement));
            } else {
                input_handler.handle_action(TextInputAction::Move(movement));
            }
        }
        KbKey::ArrowUp => {
            let movement = TextMovement::Vertical(VerticalMovement::LineUp);
            if event.mods.shift() {
                input_handler.handle_action(TextInputAction::MoveExtent(movement));
            } else {
                input_handler.handle_action(TextInputAction::Move(movement));
            }
        }
        KbKey::ArrowDown => {
            let movement = TextMovement::Vertical(VerticalMovement::LineDown);
            if event.mods.shift() {
                input_handler.handle_action(TextInputAction::MoveExtent(movement));
            } else {
                input_handler.handle_action(TextInputAction::Move(movement));
            }
        }
        _ => return false,
    };
    true
}

/// Indicates a movement that transforms a particular text position in a document.
/// These movements transform only single indices — not selections.
///
/// You'll note that a lot of these operations are idempotent, but you can get around this by first
/// sending a `Grapheme` movement.
/// If for instance, you want a `ParagraphStart` that is not idempotent, you can first send
/// `TextMovement::Grapheme(TextDirection::Upstream)`, and then follow it with `ParagraphStart`.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextMovement {
    /// A movement that stops when it reaches an extended grapheme cluster boundary.
    ///
    /// This movement is achieved on most systems by pressing the left and right arrow keys.
    /// For more information on grapheme clusters, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries).
    Grapheme(TextDirection),
    /// A movement that stops when it reaches a word boundary.
    ///
    /// This movement is achieved on most systems by pressing the left and right arrow keys while holding control.
    /// For more information on words, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Word_Boundaries).
    Word(TextDirection),
    /// A movement that stops when it reaches a soft line break.
    ///
    /// This movement is achieved on macOS by pressing the left and right arrow keys while holding command.
    /// `Line` should be idempotent: if the position is already at the end of a soft-wrapped line,
    /// this movement should never push it onto another soft-wrapped line.
    ///
    /// In order to implement this properly, your text positions should remember their affinity.
    Line(TextDirection),
    /// An upstream movement that stops when it reaches a hard line break.
    ///
    /// `ParagraphStart` should be idempotent: if the position is already at the start of a hard-wrapped line,
    /// this movement should never push it onto the previous line.
    ParagraphStart,
    /// A downstream movement that stops when it reaches a hard line break.
    ///
    /// `ParagraphEnd` should be idempotent: if the position is already at the end of a hard-wrapped line,
    /// this movement should never push it onto the next line.
    ParagraphEnd,
    /// A vertical movement, see `VerticalMovement` for more details.
    Vertical(VerticalMovement),
}

/// Indicates a horizontal direction in the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextDirection {
    /// Indicates the direction visually to the left. This may be byte-wise forwards or backwards in the document,
    /// depending on the text direction around the position being moved.
    Left,
    /// Indicates the direction visually to the right. This may be byte-wise forwards or backwards in the document,
    /// depending on the text direction around the position being moved.
    Right,
    /// Byte-wise backwards in the document. In a left-to-right context, this value is the same as `Left`.
    Upstream,
    /// Byte-wise forwards in the document. In a left-to-right context, this value is the same as `Right`.
    Downstream,
}

/// Sometimes, a byte location in a document has two visual locations. For example, the end of a soft-wrapped line
/// and the start of the subsequent line have different visual locations (and we want to be able to place an input caret
/// in either place!) but the same byte-wise location. This also shows up in bidirectional text contexts. Affinity
/// allows us to disambiguate between these two visual locations.
pub enum Affinity {
    Upstream,
    Downstream,
}

/// Indicates a horizontal direction for writing text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
    /// Indicates writing direction should be automatically detected based on the text contents.
    Natural,
}

/// Indicates a vertical movement in a text document.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VerticalMovement {
    LineUp,
    LineDown,
    PageUp,
    PageDown,
    DocumentStart,
    DocumentEnd,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextInputAction {
    /// Moves the selection.
    ///
    /// Before moving, if the extent and the anchor of the selection are not at the same
    /// position (it's a non-caret selection), then:
    ///
    /// 1. First set both extent and anchor to the same position: the selection's
    /// upstream index if `TextMovement` is an upstream movement, or the downstream index if
    /// `TextMovement` is a downstream movement.
    ///
    /// 2. If `TextMovement` is `Grapheme`, then stop. Otherwise, apply the `TextMovement` as
    /// per the usual rules.
    Move(TextMovement),

    /// Moves just the selection's extent.
    ///
    /// Equivalent to holding shift while performing movements or clicks on most operating systems.
    MoveExtent(TextMovement),

    /// Select the entire document.
    SelectAll,

    /// Expands the selection to the entire soft-wrapped line.
    ///
    /// If multiple lines are already selected, expands the selection to encompass all soft-wrapped lines
    /// that intersected with the prior selection.
    /// If the selection is a caret is on a soft line break, uses the affinity of the caret to determine
    /// which of the two lines to select.
    /// `SelectLine` should be idempotent: it should never expand onto adjacent lines.
    SelectLine,

    /// Expands the selection to the entire hard-wrapped line.
    ///
    /// If multiple lines are already selected, expands the selection to encompass all hard-wrapped lines
    /// that intersected with the prior selection.
    /// `SelectParagraph` should be idempotent: it should never expand onto adjacent lines.
    SelectParagraph,

    /// Expands the selection to the entire word.
    ///
    /// If multiple words are already selected, expands the selection to encompass all words
    /// that intersected with the prior selection.
    /// If the selection is a caret is on a word boundary, selects the word downstream of the caret.
    /// `SelectWord` should be idempotent: it should never expand onto adjacent words.
    ///
    /// For more information on what these so-called "words" are, see
    /// [Unicode Text Segmentation](https://unicode.org/reports/tr29/#Word_Boundaries).
    SelectWord,

    /// Deletes some text.
    ///
    /// If some text is already selected, `TextMovement` is ignored, and the selection is deleted.
    /// If the selection's anchor is the same as the extent, then first apply `MoveExtent(TextMovement)`
    /// and then delete the resulting selection.
    Delete(TextMovement),

    /// A special kind of backspace that, instead of deleting the entire grapheme upstream of the caret,
    /// may in some cases and character sets delete a subset of that grapheme's code points.
    DecomposingBackspace,

    /// Maps the characters in the selection to uppercase.
    ///
    /// For more information on case mapping, see the [Unicode Case Mapping FAQ](https://unicode.org/faq/casemap_charprop.html#7)
    UppercaseSelection,

    /// Maps the characters in the selection to lowercase.
    ///
    /// For more information on case mapping, see the [Unicode Case Mapping FAQ](https://unicode.org/faq/casemap_charprop.html#7)
    LowercaseSelection,

    /// Maps the characters in the selection to titlecase. When calculating whether a character is at the beginning of a word, you
    /// may have to peek outside the selection to other characters in the document.
    ///
    /// For more information on case mapping, see the [Unicode Case Mapping FAQ](https://unicode.org/faq/casemap_charprop.html#7)
    TitlecaseSelection,

    /// Inserts a newline character into the document.
    InsertNewLine {
        /// If `true`, then actually insert a newline, even if normally you would run a keyboard shortcut attached
        /// to the return key, like sending a message or activating autocomplete. On macOS, this is triggered by pressing option-return.
        ignore_hotkey: bool,
        /// Either `U+000A`, `U+2029`, or `U+2028`. For instance, on macOS, control-enter inserts `U+2028`.
        newline_type: char,
    },

    /// Inserts a tab character into the document.
    InsertTab {
        /// If `true`, then actually insert a tab, even if normally you would run a keyboard shortcut attached
        /// to the return key, like indenting a line or activating autocomplete. On macOS, this is triggered by pressing option-tab.
        ignore_hotkey: bool,
    },

    /// Indicates the reverse of inserting tab; corresponds to shift-tab on most operating systems.
    InsertBacktab,

    InsertSingleQuoteIgnoringSmartQuotes,
    InsertDoubleQuoteIgnoringSmartQuotes,

    /// Scrolls the text field without modifying the selection.
    Scroll(VerticalMovement),

    /// Centers the selection vertically in the text field. The average of the anchor's y and the extent's y should be
    /// exactly halfway down the field.
    /// If the selection is taller than the text field's visible height, then instead
    /// scrolls the minimum distance such that the text field is completely vertically filled by the selection.
    ScrollToSelection,

    /// Sets the writing direction of the selected text or caret.
    SetSelectionWritingDirection(WritingDirection),

    /// Sets the writing direction of all paragraphs that partially or fully intersect with the selection or caret.
    SetParagraphWritingDirection(WritingDirection),

    /// Cancels the current window or operation. Triggered on most operating systems with escape.
    Cancel,
}
