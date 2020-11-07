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

// massive thanks to github.com/yvt and their project tcw3 (https://github.com/yvt/Stella2), which is some
// of the most solid/well documented input method code for mac written in Rust that we've found. tcw3 was
// instrumental in this file's implementation.

// apple's documentation on text editing is also very helpful:
// https://developer.apple.com/library/archive/documentation/TextFonts/Conceptual/CocoaTextArchitecture/TextEditing/TextEditing.html#//apple_ref/doc/uid/TP40009459-CH3-SW3

#![allow(non_snake_case)]

use std::ffi::c_void;
use std::ops::Range;
use std::os::raw::c_uchar;

use super::window::get_edit_lock_from_window;
use crate::kurbo::Point;
use crate::text_input::{
    Affinity, TextDirection, TextInputAction, TextInputHandler, TextMovement, TextSelection,
    VerticalMovement, WritingDirection,
};
use cocoa::base::{id, nil, BOOL};
use cocoa::foundation::{NSArray, NSPoint, NSRect, NSSize, NSString, NSUInteger};
use cocoa::{appkit::NSWindow, foundation::NSNotFound};
use objc::runtime::{Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

// thanks to winit for the custom NSRange code:
// https://github.com/rust-windowing/winit/pull/518/files#diff-61be96e960785f102cb20ad8464eafeb6edd4245ea40224b3c3206c72cd5bf56R12-R34
#[repr(C)]
#[derive(Debug)]
pub struct NSRange {
    pub location: NSUInteger,
    pub length: NSUInteger,
}
impl NSRange {
    pub const NONE: NSRange = NSRange::new(NSNotFound as NSUInteger, 0);
    #[inline]
    pub const fn new(location: NSUInteger, length: NSUInteger) -> NSRange {
        NSRange { location, length }
    }
}
unsafe impl objc::Encode for NSRange {
    fn encode() -> objc::Encoding {
        let encoding = format!(
            "{{NSRange={}{}}}",
            NSUInteger::encode().as_str(),
            NSUInteger::encode().as_str(),
        );
        unsafe { objc::Encoding::from_str(&encoding) }
    }
}

pub extern "C" fn has_marked_text(this: &mut Object, _: Sel) -> BOOL {
    get_edit_lock_from_window(this, false)
        .map(|mut edit_lock| edit_lock.composition_range().is_some())
        .unwrap_or(false)
        .into()
}

pub extern "C" fn marked_range(this: &mut Object, _: Sel) -> NSRange {
    get_edit_lock_from_window(this, false)
        .and_then(|mut edit_lock| {
            edit_lock
                .composition_range()
                .map(|range| encode_nsrange(&mut edit_lock, range))
        })
        .unwrap_or(NSRange::NONE)
}

pub extern "C" fn selected_range(this: &mut Object, _: Sel) -> NSRange {
    let mut edit_lock = match get_edit_lock_from_window(this, false) {
        Some(v) => v,
        None => return NSRange::NONE,
    };
    let range = edit_lock.selection().to_range();
    encode_nsrange(&mut edit_lock, range)
}

pub extern "C" fn set_marked_text(
    this: &mut Object,
    _: Sel,
    text: id,
    selected_range: NSRange,
    replacement_range: NSRange,
) {
    let mut edit_lock = match get_edit_lock_from_window(this, false) {
        Some(v) => v,
        None => return,
    };
    let mut composition_range = edit_lock.composition_range().unwrap_or_else(|| {
        // no existing composition range? default to replacement range, interpreted in absolute coordinates
        // undocumented by apple, see
        // https://github.com/yvt/Stella2/blob/076fb6ee2294fcd1c56ed04dd2f4644bf456e947/tcw3/pal/src/macos/window.rs#L1144-L1146
        decode_nsrange(&mut edit_lock, &replacement_range, 0).unwrap_or_else(|| {
            // no replacement range either? apparently we default to the selection in this case
            edit_lock.selection().to_range()
        })
    });

    let replace_range_offset = edit_lock
        .composition_range()
        .map(|range| range.start)
        .unwrap_or(0);

    let replace_range = decode_nsrange(&mut edit_lock, &replacement_range, replace_range_offset)
        .unwrap_or_else(|| {
            // default replacement range is already-exsiting composition range
            // undocumented by apple, see
            // https://github.com/yvt/Stella2/blob/076fb6ee2294fcd1c56ed04dd2f4644bf456e947/tcw3/pal/src/macos/window.rs#L1124-L1125
            composition_range.clone()
        });

    let text_string = parse_attributed_string(&text);
    edit_lock.replace_range(replace_range.clone(), text_string);

    // Update the composition range
    composition_range.end -= replace_range.len();
    composition_range.end += text_string.len();
    if composition_range.is_empty() {
        edit_lock.set_composition_range(None);
    } else {
        edit_lock.set_composition_range(Some(composition_range));
    };

    // Update the selection
    if let Some(selection_range) =
        decode_nsrange(&mut edit_lock, &selected_range, replace_range.start)
    {
        // preserve ordering of anchor and extent
        let existing_selection = edit_lock.selection();
        let new_selection = if existing_selection.anchor < existing_selection.extent {
            TextSelection {
                anchor: selection_range.start,
                extent: selection_range.end,
            }
        } else {
            TextSelection {
                extent: selection_range.start,
                anchor: selection_range.end,
            }
        };
        edit_lock.set_selection(new_selection);
    }
}

pub extern "C" fn unmark_text(this: &mut Object, _: Sel) {
    let mut edit_lock = match get_edit_lock_from_window(this, false) {
        Some(v) => v,
        None => return,
    };
    edit_lock.set_composition_range(None);
}

pub extern "C" fn valid_attributes_for_marked_text(_this: &mut Object, _: Sel) -> id {
    // we don't support any attributes
    unsafe { NSArray::array(nil) }
}

pub extern "C" fn attributed_substring_for_proposed_range(
    this: &mut Object,
    _: Sel,
    proposed_range: NSRange,
    actual_range: *mut c_void,
) -> id {
    let mut edit_lock = match get_edit_lock_from_window(this, false) {
        Some(v) => v,
        None => return nil,
    };
    let range = match decode_nsrange(&mut edit_lock, &proposed_range, 0) {
        Some(v) => v,
        None => return nil,
    };
    if !actual_range.is_null() {
        let ptr = actual_range as *mut NSRange;
        let range_utf16 = encode_nsrange(&mut edit_lock, range.clone());
        unsafe {
            *ptr = range_utf16;
        }
    }
    let text = edit_lock.slice(range);
    unsafe {
        let ns_string = NSString::alloc(nil).init_str(&text);
        let attr_string: id = msg_send![class!(NSAttributedString), alloc];
        msg_send![attr_string, initWithString: ns_string]
    }
}

pub extern "C" fn insert_text(this: &mut Object, _: Sel, text: id, replacement_range: NSRange) {
    let mut edit_lock = match get_edit_lock_from_window(this, true) {
        Some(v) => v,
        None => return,
    };
    let text_string = parse_attributed_string(&text);

    // yvt notes:
    // [The null range case] is undocumented, but it seems that it means
    // the whole marked text or selected text should be finalized
    // and replaced with the given string.
    // https://github.com/yvt/Stella2/blob/076fb6ee2294fcd1c56ed04dd2f4644bf456e947/tcw3/pal/src/macos/window.rs#L1041-L1043
    let converted_range = decode_nsrange(&mut edit_lock, &replacement_range, 0)
        .or_else(|| edit_lock.composition_range())
        .unwrap_or_else(|| edit_lock.selection().to_range());

    edit_lock.replace_range(converted_range.clone(), text_string);
    edit_lock.set_composition_range(None);
    // move the caret next to the inserted text
    let caret_index = converted_range.start + text_string.len();
    edit_lock.set_selection(TextSelection::new_caret(caret_index));
}

pub extern "C" fn character_index_for_point(
    this: &mut Object,
    _: Sel,
    point: NSPoint,
) -> NSUInteger {
    let mut edit_lock = match get_edit_lock_from_window(this, true) {
        Some(v) => v,
        None => return 0,
    };
    let hit_test = edit_lock.hit_test_point(Point::new(point.x, point.y));
    hit_test.idx as NSUInteger
}

pub extern "C" fn first_rect_for_character_range(
    this: &mut Object,
    _: Sel,
    character_range: NSRange,
    actual_range: *mut c_void,
) -> NSRect {
    let mut edit_lock = match get_edit_lock_from_window(this, true) {
        Some(v) => v,
        None => return NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
    };
    let mut range = decode_nsrange(&mut edit_lock, &character_range, 0).unwrap_or(0..0);
    {
        let line_range = edit_lock.line_range(range.start, Affinity::Downstream);
        range.end = usize::min(range.end, line_range.end);
    }
    let rect = match edit_lock.slice_bounding_box(range.clone()) {
        Some(v) => v,
        None => return NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
    };
    if !actual_range.is_null() {
        let ptr = actual_range as *mut NSRange;
        let range_utf16 = encode_nsrange(&mut edit_lock, range);
        unsafe {
            *ptr = range_utf16;
        }
    }
    let view_space_rect = NSRect::new(
        NSPoint::new(rect.x0, rect.y0),
        NSSize::new(rect.width(), rect.height()),
    );
    unsafe {
        let window_space_rect: NSRect =
            msg_send![this as *const _, convertRect: view_space_rect toView: nil];
        let window: id = msg_send![this as *const _, window];
        window.convertRectToScreen_(window_space_rect)
    }
}

pub extern "C" fn do_command_by_selector(this: &mut Object, _: Sel, cmd: Sel) {
    let mut edit_lock = match get_edit_lock_from_window(this, true) {
        Some(v) => v,
        None => return,
    };
    match cmd.name() {
        // see https://developer.apple.com/documentation/appkit/nsstandardkeybindingresponding?language=objc
        // and https://support.apple.com/en-us/HT201236
        // and https://support.apple.com/lv-lv/guide/mac-help/mh21243/mac
        "centerSelectionInVisibleArea:" => {
            edit_lock.handle_action(TextInputAction::ScrollToSelection)
        }
        "deleteBackward:" => edit_lock.handle_action(TextInputAction::Delete(
            TextMovement::Grapheme(TextDirection::Upstream),
        )),
        "deleteBackwardByDecomposingPreviousCharacter:" => {
            edit_lock.handle_action(TextInputAction::DecomposingBackspace)
        }
        "deleteForward:" => edit_lock.handle_action(TextInputAction::Delete(
            TextMovement::Grapheme(TextDirection::Downstream),
        )),
        "deleteToBeginningOfLine:" => edit_lock.handle_action(TextInputAction::Delete(
            TextMovement::Line(TextDirection::Upstream),
        )),
        "deleteToBeginningOfParagraph:" => {
            // TODO(lord): this seems to also kill the text into a buffer that can get pasted with yank
            edit_lock.handle_action(TextInputAction::Delete(TextMovement::ParagraphStart))
        }
        "deleteToEndOfLine:" => edit_lock.handle_action(TextInputAction::Delete(
            TextMovement::Line(TextDirection::Downstream),
        )),
        "deleteToEndOfParagraph:" => {
            // TODO(lord): this seems to also kill the text into a buffer that can get pasted with yank
            edit_lock.handle_action(TextInputAction::Delete(TextMovement::ParagraphEnd))
        }
        "deleteWordBackward:" => edit_lock.handle_action(TextInputAction::Delete(
            TextMovement::Word(TextDirection::Upstream),
        )),
        "deleteWordForward:" => edit_lock.handle_action(TextInputAction::Delete(
            TextMovement::Word(TextDirection::Downstream),
        )),
        "insertBacktab:" => edit_lock.handle_action(TextInputAction::InsertBacktab),
        "insertLineBreak:" => edit_lock.handle_action(TextInputAction::InsertNewLine {
            ignore_hotkey: false,
            newline_type: '\u{2028}',
        }),
        "insertNewline:" => edit_lock.handle_action(TextInputAction::InsertNewLine {
            ignore_hotkey: false,
            newline_type: '\n',
        }),
        "insertNewlineIgnoringFieldEditor:" => {
            edit_lock.handle_action(TextInputAction::InsertNewLine {
                ignore_hotkey: true,
                newline_type: '\n',
            })
        }
        "insertParagraphSeparator:" => edit_lock.handle_action(TextInputAction::InsertNewLine {
            ignore_hotkey: false,
            newline_type: '\u{2029}',
        }),
        "insertTab:" => edit_lock.handle_action(TextInputAction::InsertTab {
            ignore_hotkey: false,
        }),
        "insertTabIgnoringFieldEditor:" => edit_lock.handle_action(TextInputAction::InsertTab {
            ignore_hotkey: true,
        }),
        "makeBaseWritingDirectionLeftToRight:" => edit_lock.handle_action(
            TextInputAction::SetParagraphWritingDirection(WritingDirection::LeftToRight),
        ),
        "makeBaseWritingDirectionNatural:" => edit_lock.handle_action(
            TextInputAction::SetParagraphWritingDirection(WritingDirection::Natural),
        ),
        "makeBaseWritingDirectionRightToLeft:" => edit_lock.handle_action(
            TextInputAction::SetParagraphWritingDirection(WritingDirection::RightToLeft),
        ),
        "makeTextWritingDirectionLeftToRight:" => edit_lock.handle_action(
            TextInputAction::SetSelectionWritingDirection(WritingDirection::LeftToRight),
        ),
        "makeTextWritingDirectionNatural:" => edit_lock.handle_action(
            TextInputAction::SetSelectionWritingDirection(WritingDirection::Natural),
        ),
        "makeTextWritingDirectionRightToLeft:" => edit_lock.handle_action(
            TextInputAction::SetSelectionWritingDirection(WritingDirection::RightToLeft),
        ),
        "moveBackward:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Grapheme(
            TextDirection::Upstream,
        ))),
        "moveBackwardAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Grapheme(TextDirection::Upstream),
        )),
        "moveDown:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Vertical(
            VerticalMovement::LineDown,
        ))),
        "moveDownAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Vertical(VerticalMovement::LineDown),
        )),
        "moveForward:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Grapheme(
            TextDirection::Downstream,
        ))),
        "moveForwardAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Grapheme(TextDirection::Downstream),
        )),
        "moveLeft:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Grapheme(
            TextDirection::Left,
        ))),
        "moveLeftAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Grapheme(TextDirection::Left),
        )),
        "moveParagraphBackwardAndModifySelection:" => {
            let selection = edit_lock.selection();
            let is_extent_after_anchor = selection.extent > selection.anchor;
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::Grapheme(
                TextDirection::Upstream,
            )));
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::ParagraphStart));
            if is_extent_after_anchor && selection.extent <= selection.anchor {
                // textedit testing showed that this operation never fully inverts a selection; if this action
                // would cause a selection's extent and anchor to swap order, it makes a caret instead. applying
                // the operation a second time (on the selection that is now a caret) is required to invert.
                edit_lock.set_selection(TextSelection::new_caret(selection.anchor));
            }
        }
        "moveParagraphForwardAndModifySelection:" => {
            let selection = edit_lock.selection();
            let is_anchor_after_extent = selection.extent < selection.anchor;
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::Grapheme(
                TextDirection::Downstream,
            )));
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::ParagraphEnd));
            if is_anchor_after_extent && selection.extent >= selection.anchor {
                // textedit testing showed that this operation never fully inverts a selection; if this action
                // would cause a selection's extent and anchor to swap order, it makes a caret instead. applying
                // the operation a second time (on the selection that is now a caret) is required to invert.
                edit_lock.set_selection(TextSelection::new_caret(selection.anchor));
            }
        }
        "moveRight:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Grapheme(
            TextDirection::Right,
        ))),
        "moveRightAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Grapheme(TextDirection::Right),
        )),
        "moveToBeginningOfDocument:" => edit_lock.handle_action(TextInputAction::Move(
            TextMovement::Vertical(VerticalMovement::DocumentStart),
        )),
        "moveToBeginningOfDocumentAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Vertical(VerticalMovement::DocumentStart)),
        ),
        "moveToBeginningOfLine:" => edit_lock.handle_action(TextInputAction::Move(
            TextMovement::Line(TextDirection::Upstream),
        )),
        "moveToBeginningOfLineAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Line(TextDirection::Upstream)),
        ),
        "moveToBeginningOfParagraph:" => {
            // initially, it may seem like this and moveToEndOfParagraph shouldn't be idempotent. after all,
            // option-up and option-down aren't idempotent, and those seem to call this method. however, on
            // further inspection, you can find that option-up first calls `moveBackward` before calling this.
            // if you send the raw command to TextEdit by editing your `DefaultKeyBinding.dict`, you can find
            // that this operation is in fact idempotent.
            edit_lock.handle_action(TextInputAction::Move(TextMovement::ParagraphStart))
        }
        "moveToBeginningOfParagraphAndModifySelection:" => {
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::ParagraphStart))
        }
        "moveToEndOfDocument:" => edit_lock.handle_action(TextInputAction::Move(
            TextMovement::Vertical(VerticalMovement::DocumentEnd),
        )),
        "moveToEndOfDocumentAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Vertical(VerticalMovement::DocumentEnd)),
        ),
        "moveToEndOfLine:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Line(
            TextDirection::Downstream,
        ))),
        "moveToEndOfLineAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Line(TextDirection::Downstream)),
        ),
        "moveToEndOfParagraph:" => {
            edit_lock.handle_action(TextInputAction::Move(TextMovement::ParagraphEnd))
        }
        "moveToEndOfParagraphAndModifySelection:" => {
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::ParagraphEnd))
        }
        "moveToLeftEndOfLine:" => edit_lock.handle_action(TextInputAction::Move(
            TextMovement::Line(TextDirection::Left),
        )),
        "moveToLeftEndOfLineAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Line(TextDirection::Left)),
        ),
        "moveToRightEndOfLine:" => edit_lock.handle_action(TextInputAction::Move(
            TextMovement::Line(TextDirection::Right),
        )),
        "moveToRightEndOfLineAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Line(TextDirection::Right)),
        ),
        "moveUp:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Vertical(
            VerticalMovement::LineUp,
        ))),
        "moveUpAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Vertical(VerticalMovement::LineUp),
        )),
        "moveWordBackward:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Word(
            TextDirection::Upstream,
        ))),
        "moveWordBackwardAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Word(TextDirection::Upstream)),
        ),
        "moveWordForward:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Word(
            TextDirection::Downstream,
        ))),
        "moveWordForwardAndModifySelection:" => edit_lock.handle_action(
            TextInputAction::MoveExtent(TextMovement::Word(TextDirection::Downstream)),
        ),
        "moveWordLeft:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Word(
            TextDirection::Left,
        ))),
        "moveWordLeftAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Word(TextDirection::Left),
        )),
        "moveWordRight:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Word(
            TextDirection::Right,
        ))),
        "moveWordRightAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Word(TextDirection::Right),
        )),
        "pageDown:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Vertical(
            VerticalMovement::PageDown,
        ))),
        "pageDownAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Vertical(VerticalMovement::PageDown),
        )),
        "pageUp:" => edit_lock.handle_action(TextInputAction::Move(TextMovement::Vertical(
            VerticalMovement::PageUp,
        ))),
        "pageUpAndModifySelection:" => edit_lock.handle_action(TextInputAction::MoveExtent(
            TextMovement::Vertical(VerticalMovement::PageUp),
        )),
        "scrollLineDown:" => {
            edit_lock.handle_action(TextInputAction::Scroll(VerticalMovement::LineDown))
        }
        "scrollLineUp:" => {
            edit_lock.handle_action(TextInputAction::Scroll(VerticalMovement::LineUp))
        }
        "scrollPageDown:" => {
            edit_lock.handle_action(TextInputAction::Scroll(VerticalMovement::PageDown))
        }
        "scrollPageUp:" => {
            edit_lock.handle_action(TextInputAction::Scroll(VerticalMovement::PageUp))
        }
        "scrollToBeginningOfDocument:" => {
            edit_lock.handle_action(TextInputAction::Scroll(VerticalMovement::DocumentStart))
        }
        "scrollToEndOfDocument:" => {
            edit_lock.handle_action(TextInputAction::Scroll(VerticalMovement::DocumentEnd))
        }
        "selectAll:" => edit_lock.handle_action(TextInputAction::SelectAll),
        "selectLine:" => edit_lock.handle_action(TextInputAction::SelectLine),
        "selectParagraph:" => edit_lock.handle_action(TextInputAction::SelectParagraph),
        "selectWord:" => edit_lock.handle_action(TextInputAction::SelectWord),
        "transpose:" => {
            // transpose is a tricky-to-implement-correctly mac-specific operation, and so we implement it using
            // edit_lock commands directly instead of allowing applications to implement it directly through an
            // action handler. it seems extremely unlikely anybody would want to override its behavior, anyway.

            // Swaps the graphemes before and after the caret, and then moves the caret downstream one grapheme. If the caret is at the
            // end of a hard-wrapped line or document, act as though the caret was one grapheme upstream instead. If the caret is at the
            // beginning of the document, this is a no-op. This is a no-op on non-caret selections (when `anchor != extent`).

            {
                let selection = edit_lock.selection();
                if !selection.is_caret() || selection.anchor == 0 {
                    return;
                }
            }

            // move caret to the end of the transpose region
            {
                let old_selection = edit_lock.selection();
                edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::Grapheme(
                    TextDirection::Downstream,
                )));
                let new_selection = edit_lock.selection().to_range();
                let next_grapheme = edit_lock.slice(new_selection.clone());
                let next_char = next_grapheme.chars().next();

                if next_char == Some('\n')
                    || next_char == Some('\r')
                    || next_char == Some('\u{2029}')
                    || next_char == Some('\u{2028}')
                    || next_char == None
                {
                    // next char is a newline or end of doc; so end of transpose range will actually be the starting selection.anchor
                    edit_lock.set_selection(old_selection);
                } else {
                    // normally, end of transpose range will be next grapheme
                    edit_lock.set_selection(TextSelection::new_caret(new_selection.end));
                }
            }

            // now find the previous two graphemes
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::Grapheme(
                TextDirection::Upstream,
            )));
            let middle_idx = edit_lock.selection().extent;
            edit_lock.handle_action(TextInputAction::MoveExtent(TextMovement::Grapheme(
                TextDirection::Upstream,
            )));
            let selection = edit_lock.selection();
            let first_grapheme = edit_lock
                .slice(selection.upstream_index()..middle_idx)
                .into_owned();
            let second_grapheme = edit_lock.slice(middle_idx..selection.downstream_index());
            let new_string = format!("{}{}", second_grapheme, first_grapheme);
            // replace_range should automatically set selection to end of inserted range
            edit_lock.replace_range(selection.to_range(), &new_string);
        }
        "capitalizeWord:" => {
            // this command expands the selection to words, and then applies titlecase to that selection
            // not actually sure what keyboard shortcut corresponds to this or the other case changing commands,
            // but textedit seems to have this behavior when this action is simulated by editing DefaultKeyBinding.dict.
            edit_lock.handle_action(TextInputAction::SelectWord);
            edit_lock.handle_action(TextInputAction::TitlecaseSelection)
        }
        "lowercaseWord:" => {
            // this command expands the selection to words, and then applies uppercase to that selection
            edit_lock.handle_action(TextInputAction::SelectWord);
            edit_lock.handle_action(TextInputAction::LowercaseSelection);
        }
        "uppercaseWord:" => {
            // this command expands the selection to words, and then applies uppercase to that selection
            edit_lock.handle_action(TextInputAction::SelectWord);
            edit_lock.handle_action(TextInputAction::UppercaseSelection);
        }
        "insertDoubleQuoteIgnoringSubstitution:" => {
            edit_lock.handle_action(TextInputAction::InsertDoubleQuoteIgnoringSmartQuotes)
        }
        "insertSingleQuoteIgnoringSubstitution:" => {
            edit_lock.handle_action(TextInputAction::InsertSingleQuoteIgnoringSmartQuotes)
        }
        "cancelOperation:" => edit_lock.handle_action(TextInputAction::Cancel),
        // "deleteToMark:" => {}          // TODO(lord): selectToMark, then delete selection. also puts selection in yank buffer
        // "selectToMark:" => {}          // TODO(lord): extends the selection to include the mark
        // "setMark:" => {}               // TODO(lord): remembers index in document (but what about grapheme clusters??)
        // "swapWithMark:" => {}          // TODO(lord): swaps current and stored mark indices
        // "yank:" => {}                  // TODO(lord): triggered with control-y, inserts in the text previously killed deleteTo*OfParagraph
        "transposeWords:" => {} // textedit doesn't support, so neither will we
        "changeCaseOfLetter:" => {} // textedit doesn't support, so neither will we
        "indent:" => {}         // textedit doesn't support, so neither will we
        "insertContainerBreak:" => {} // textedit and pages don't seem to support, so neither will we
        "quickLookPreviewItems:" => {} // don't seem to apply to text editing?? hopefully
        "complete:" => {}             // intercepted by the IME; if it gets to us, noop
        "noop:" => {}
        e => {
            eprintln!("unknown text editing command from macos: {}", e);
        }
    };
}

/// Parses the UTF-16 `NSRange` into a UTF-8 `Range<usize>`.
/// `start_offset` is the UTF-8 offset into the document that `range` values are relative to. Set it to `0` if `range`
/// is absolute instead of relative.
/// Returns `None` if `range` was invalid; macOS often uses this to indicate some special null value.
fn decode_nsrange(
    edit_lock: &mut Box<dyn TextInputHandler>,
    range: &NSRange,
    start_offset: usize,
) -> Option<Range<usize>> {
    if range.location as usize >= i32::max_value() as usize {
        return None;
    }
    let start_offset_utf16 = edit_lock.utf8_to_utf16(0..start_offset);
    let location_utf16 = range.location as usize + start_offset_utf16;
    let length_utf16 = range.length as usize;
    let start_utf8 = edit_lock.utf16_to_utf8(0..location_utf16);
    let end_utf8 =
        start_utf8 + edit_lock.utf16_to_utf8(location_utf16..location_utf16 + length_utf16);
    Some(start_utf8..end_utf8)
}

// Encodes the UTF-8 `Range<usize>` into a UTF-16 `NSRange`.
fn encode_nsrange(edit_lock: &mut Box<dyn TextInputHandler>, mut range: Range<usize>) -> NSRange {
    while !edit_lock.is_char_boundary(range.start) {
        range.start -= 1;
    }
    while !edit_lock.is_char_boundary(range.end) {
        range.end -= 1;
    }
    let start = edit_lock.utf8_to_utf16(0..range.start);
    let len = edit_lock.utf8_to_utf16(range);
    NSRange::new(start as NSUInteger, len as NSUInteger)
}

fn parse_attributed_string(text: &id) -> &str {
    unsafe {
        let nsstring = if msg_send![*text, isKindOfClass: class!(NSAttributedString)] {
            msg_send![*text, string]
        } else {
            // already a NSString
            *text
        };
        let slice =
            std::slice::from_raw_parts(nsstring.UTF8String() as *const c_uchar, nsstring.len());
        std::str::from_utf8_unchecked(slice)
    }
}
