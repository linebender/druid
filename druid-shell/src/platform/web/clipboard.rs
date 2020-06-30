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

//! Interactions with the browser pasteboard.

use crate::clipboard::{ClipboardFormat, FormatId};

/// The browser clipboard.
#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, _s: impl AsRef<str>) {
        log::warn!("unimplemented");
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, _formats: &[ClipboardFormat]) {
        log::warn!("unimplemented");
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        log::warn!("unimplemented");
        None
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, _formats: &[FormatId]) -> Option<FormatId> {
        log::warn!("unimplemented");
        None
    }

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    pub fn get_format(&self, _format: FormatId) -> Option<Vec<u8>> {
        log::warn!("unimplemented");
        None
    }

    pub fn available_type_names(&self) -> Vec<String> {
        log::warn!("unimplemented");
        Vec::new()
    }
}
