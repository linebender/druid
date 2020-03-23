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

//! Calc start of a backspace delete interval

use crate::text::{EditableText, EditableTextCursor, Selection};

use xi_unicode::*;

#[derive(PartialEq)]
enum CodepointState {
    Start,
    Lf,
    BeforeKeycap,
    BeforeVsAndKeycap,
    BeforeEmojiModifier,
    BeforeVSAndEmojiModifier,
    BeforeVS,
    BeforeEmoji,
    BeforeZwj,
    BeforeVSAndZWJ,
    BeforeWhitespace,
    BeforeNonSpace,
    OddNumberedRIS,
    EvenNumberedRIS,
    InTagSequence,
    Finished,
}

fn backspace_word_offset(text: &impl EditableText, start: usize) -> usize {
    let mut state = CodepointState::Start;
    let mut delete_code_point_count = 0;
    let mut delete_from_whitespace = false;
    let mut cursor = text
        .cursor(start)
        .expect("Backspace must begin at a valid codepoint boundary.");
    while state != CodepointState::Finished && cursor.pos() > 0 {
        let code_point = cursor.prev_codepoint().unwrap_or('0');
        match state {
            CodepointState::Start => {
                delete_code_point_count += 1;
                if code_point == '\n' {
                    state = CodepointState::Lf;
                } else if is_whitespace(&code_point) {
                    delete_from_whitespace = true;
                    state = CodepointState::BeforeWhitespace;
                } else {
                    delete_from_whitespace = false;
                    state = CodepointState::BeforeNonSpace;
                }
            }
            CodepointState::Lf => {
                if code_point == '\r' {
                    delete_code_point_count += 1;
                }
                state = CodepointState::Finished;
            }
            CodepointState::BeforeWhitespace => {
                if is_whitespace(&code_point) && delete_from_whitespace {
                    delete_code_point_count += 1;
                } else {
                    state = CodepointState::Finished
                }
            }
            CodepointState::BeforeNonSpace => {
                if is_whitespace(&code_point) {
                    delete_code_point_count += 1;
                    state = CodepointState::Finished;
                } else {
                    delete_code_point_count += 1;
                }
            }
            CodepointState::Finished => {
                break;
            }
            _ => {}
        }
    }
    cursor.set(start);
    for _ in 0..delete_code_point_count {
        let _ = cursor.prev_codepoint();
    }
    cursor.pos()
}

#[allow(clippy::cognitive_complexity)]
fn backspace_offset(text: &impl EditableText, start: usize) -> usize {
    let mut state = CodepointState::Start;

    let mut delete_code_point_count = 0;
    let mut last_seen_vs_code_point_count = 0;
    let mut cursor = text
        .cursor(start)
        .expect("Backspace must begin at a valid codepoint boundary.");

    while state != CodepointState::Finished && cursor.pos() > 0 {
        let code_point = cursor.prev_codepoint().unwrap_or('0');

        match state {
            CodepointState::Start => {
                delete_code_point_count = 1;
                if code_point == '\n' {
                    state = CodepointState::Lf;
                } else if is_variation_selector(code_point) {
                    state = CodepointState::BeforeVS;
                } else if code_point.is_regional_indicator_symbol() {
                    state = CodepointState::OddNumberedRIS;
                } else if code_point.is_emoji_modifier() {
                    state = CodepointState::BeforeEmojiModifier;
                } else if code_point.is_emoji_combining_enclosing_keycap() {
                    state = CodepointState::BeforeKeycap;
                } else if code_point.is_emoji() {
                    state = CodepointState::BeforeEmoji;
                } else if code_point.is_emoji_cancel_tag() {
                    state = CodepointState::InTagSequence;
                } else {
                    state = CodepointState::Finished;
                }
            }
            CodepointState::Lf => {
                if code_point == '\r' {
                    delete_code_point_count += 1;
                }
                state = CodepointState::Finished;
            }
            CodepointState::OddNumberedRIS => {
                if code_point.is_regional_indicator_symbol() {
                    delete_code_point_count += 1;
                    state = CodepointState::EvenNumberedRIS
                } else {
                    state = CodepointState::Finished
                }
            }
            CodepointState::EvenNumberedRIS => {
                if code_point.is_regional_indicator_symbol() {
                    delete_code_point_count -= 1;
                    state = CodepointState::OddNumberedRIS;
                } else {
                    state = CodepointState::Finished;
                }
            }
            CodepointState::BeforeKeycap => {
                if is_variation_selector(code_point) {
                    last_seen_vs_code_point_count = 1;
                    state = CodepointState::BeforeVsAndKeycap;
                } else {
                    if is_keycap_base(code_point) {
                        delete_code_point_count += 1;
                    }
                    state = CodepointState::Finished;
                }
            }
            CodepointState::BeforeVsAndKeycap => {
                if is_keycap_base(code_point) {
                    delete_code_point_count += last_seen_vs_code_point_count + 1;
                }
                state = CodepointState::Finished;
            }
            CodepointState::BeforeEmojiModifier => {
                if is_variation_selector(code_point) {
                    last_seen_vs_code_point_count = 1;
                    state = CodepointState::BeforeVSAndEmojiModifier;
                } else {
                    if code_point.is_emoji_modifier_base() {
                        delete_code_point_count += 1;
                    }
                    state = CodepointState::Finished;
                }
            }
            CodepointState::BeforeVSAndEmojiModifier => {
                if code_point.is_emoji_modifier_base() {
                    delete_code_point_count += last_seen_vs_code_point_count + 1;
                }
                state = CodepointState::Finished;
            }
            CodepointState::BeforeVS => {
                if code_point.is_emoji() {
                    delete_code_point_count += 1;
                    state = CodepointState::BeforeEmoji;
                } else {
                    if !is_variation_selector(code_point) {
                        //TODO: UCharacter.getCombiningClass(codePoint) == 0
                        delete_code_point_count += 1;
                    }
                    state = CodepointState::Finished;
                }
            }
            CodepointState::BeforeEmoji => {
                if code_point.is_zwj() {
                    state = CodepointState::BeforeZwj;
                } else {
                    state = CodepointState::Finished;
                }
            }
            CodepointState::BeforeZwj => {
                if code_point.is_emoji() {
                    delete_code_point_count += 2;
                    state = if code_point.is_emoji_modifier() {
                        CodepointState::BeforeEmojiModifier
                    } else {
                        CodepointState::BeforeEmoji
                    };
                } else if is_variation_selector(code_point) {
                    last_seen_vs_code_point_count = 1;
                    state = CodepointState::BeforeVSAndZWJ;
                } else {
                    state = CodepointState::Finished;
                }
            }
            CodepointState::BeforeVSAndZWJ => {
                if code_point.is_emoji() {
                    delete_code_point_count += last_seen_vs_code_point_count + 2;
                    last_seen_vs_code_point_count = 0;
                    state = CodepointState::BeforeEmoji;
                } else {
                    state = CodepointState::Finished;
                }
            }
            CodepointState::InTagSequence => {
                if code_point.is_tag_spec_char() {
                    delete_code_point_count += 1;
                } else if code_point.is_emoji() {
                    delete_code_point_count += 1;
                    state = CodepointState::Finished;
                } else {
                    delete_code_point_count = 1;
                    state = CodepointState::Finished;
                }
            }
            CodepointState::Finished => {
                break;
            }
            _ => {}
        }
    }

    cursor.set(start);
    for _ in 0..delete_code_point_count {
        let _ = cursor.prev_codepoint();
    }
    cursor.pos()
}

/// Calculate resulting offset for a backwards delete.
pub fn offset_for_delete_backwards(region: &Selection, text: &impl EditableText) -> usize {
    if !region.is_caret() {
        region.min()
    } else {
        backspace_offset(text, region.end)
    }
}

/// Calculate resulting offset for a backwards word-deletion.
pub fn offset_for_delete_word_backwards(region: &Selection, text: &impl EditableText) -> usize {
    if !region.is_caret() {
        region.min()
    } else {
        backspace_word_offset(text, region.end)
    }
}

// todo: put this in xi-unicode?
// see http://jkorpela.fi/chars/spaces.html
const UNICODE_SPACES: [char; 20] = [
    '\u{0020}', '\u{00A0}', '\u{1680}', '\u{180E}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
    '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{200B}',
    '\u{202F}', '\u{205F}', '\u{3000}', '\u{FEFF}',
];

pub fn is_whitespace(c: &char) -> bool {
    UNICODE_SPACES.contains(c)
}
