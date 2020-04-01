// Copyright 2018 The xi-editor Authors.
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

//! A bunch of boilerplate for converting the `EditNotification`s we receive
//! from the client into the events we use internally.
//!
//! This simplifies code elsewhere, and makes it easier to route events to
//! the editor or view as appropriate.

use super::movement::Movement;

/// Events that only modify view state
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum ViewEvent {
    Move(Movement),
    ModifySelection(Movement),
    SelectAll,
    Scroll(LineRange),
    AddSelectionAbove,
    AddSelectionBelow,
    Gesture {
        line: u64,
        col: u64,
        ty: GestureType,
    },
}

/// Events that modify the buffer
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum BufferEvent {
    Delete { movement: Movement, kill: bool },
    Backspace,
    Transpose,
    Undo,
    Redo,
    Uppercase,
    Lowercase,
    Capitalize,
    Indent,
    Outdent,
    Insert(String),
    Paste(String),
    InsertNewline,
    InsertTab,
    Yank,
    // ReplaceNext,
    // ReplaceAll,
    DuplicateLine,
    IncreaseNumber,
    DecreaseNumber,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum EventDomain {
    View(ViewEvent),
    Buffer(BufferEvent),
}

impl From<BufferEvent> for EventDomain {
    fn from(src: BufferEvent) -> EventDomain {
        EventDomain::Buffer(src)
    }
}

impl From<ViewEvent> for EventDomain {
    fn from(src: ViewEvent) -> EventDomain {
        EventDomain::View(src)
    }
}

/// An inclusive range.
///
/// # Note:
///
/// Several core protocol commands use a params array to pass arguments
/// which are named, internally. this type use custom Serialize /
/// Deserialize impls to accommodate this.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LineRange {
    pub first: i64,
    pub last: i64,
}

/// An enum representing touch and mouse gestures applied to the text.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum GestureType {
    Select {
        granularity: SelectionGranularity,
        multi: bool,
    },
    SelectExtend {
        granularity: SelectionGranularity,
    },
    Drag,
}

/// A mouse event. See the note for [`LineRange`].
///
/// [`LineRange`]: enum.LineRange.html
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MouseAction {
    pub line: u64,
    pub column: u64,
    pub flags: u64,
    pub click_count: Option<u64>,
}

/// The smallest unit of text that a gesture can select
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum SelectionGranularity {
    /// Selects any point or character range
    Point,
    /// Selects one word at a time
    Word,
    /// Selects one line at a time
    Line,
}
