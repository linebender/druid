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

//! Interactions with the system pasteboard on X11.

use crate::clipboard::{ClipboardFormat, FormatId};

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    pub fn put_string(&mut self, _s: impl AsRef<str>) {
        // TODO(x11/clipboard): implement Clipboard::put_string
        log::warn!("Clipboard::put_string is currently unimplemented for X11 platforms.");
    }

    pub fn put_formats(&mut self, _formats: &[ClipboardFormat]) {
        // TODO(x11/clipboard): implement Clipboard::put_formats
        log::warn!("Clipboard::put_formats is currently unimplemented for X11 platforms.");
    }

    pub fn get_string(&self) -> Option<String> {
        // TODO(x11/clipboard): implement Clipboard::get_string
        log::warn!("Clipboard::set_string is currently unimplemented for X11 platforms.");
        None
    }

    pub fn preferred_format(&self, _formats: &[FormatId]) -> Option<FormatId> {
        // TODO(x11/clipboard): implement Clipboard::preferred_format
        log::warn!("Clipboard::preferred_format is currently unimplemented for X11 platforms.");
        None
    }

    pub fn get_format(&self, _format: FormatId) -> Option<Vec<u8>> {
        // TODO(x11/clipboard): implement Clipboard::get_format
        log::warn!("Clipboard::get_format is currently unimplemented for X11 platforms.");
        None
    }

    pub fn available_type_names(&self) -> Vec<String> {
        // TODO(x11/clipboard): implement Clipboard::available_type_names
        log::warn!("Clipboard::available_type_names is currently unimplemented for X11 platforms.");
        vec![]
    }
}
