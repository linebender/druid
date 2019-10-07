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

//! Interacting with the system pasteboard/clipboard.

/// An item on the system clipboard.
#[derive(Debug, Clone)]
pub enum ClipboardItem {
    Text(String),
    #[doc(hidden)]
    __NotExhaustive,
    // other things
}

impl ClipboardItem {
    /// Create a new `ClipboardItem`.
    pub fn new(item: impl Into<ClipboardItem>) -> Self {
        item.into()
    }
}

impl<T: Into<String>> From<T> for ClipboardItem {
    fn from(src: T) -> ClipboardItem {
        ClipboardItem::Text(src.into())
    }
}

//TODO: custom formats.
// https://docs.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#registered-clipboard-formats
