// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactions with the browser pasteboard.

use crate::clipboard::{ClipboardFormat, FormatId};

/// The browser clipboard.
#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, _s: impl AsRef<str>) {
        tracing::warn!("unimplemented");
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, _formats: &[ClipboardFormat]) {
        tracing::warn!("unimplemented");
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        tracing::warn!("unimplemented");
        None
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, _formats: &[FormatId]) -> Option<FormatId> {
        tracing::warn!("unimplemented");
        None
    }

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    pub fn get_format(&self, _format: FormatId) -> Option<Vec<u8>> {
        tracing::warn!("unimplemented");
        None
    }

    pub fn available_type_names(&self) -> Vec<String> {
        tracing::warn!("unimplemented");
        Vec::new()
    }
}
