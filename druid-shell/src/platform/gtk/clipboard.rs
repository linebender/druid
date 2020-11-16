// Copyright 2019 The Druid Authors.
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

//! Interactions with the system pasteboard on GTK+.

use gdk::Atom;
use gtk::{TargetEntry, TargetFlags};

use crate::clipboard::{ClipboardFormat, FormatId};

/// The system clipboard.
#[derive(Debug, Clone)]
pub struct Clipboard;

impl Clipboard {
    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, s: impl AsRef<str>) {
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();
        clipboard.set_text(s.as_ref())
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        let entries = make_entries(formats);
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();
        // this is gross: we need to reclone all the data in formats in order
        // to move it into the closure. :/
        let formats = formats.to_owned();
        let success = clipboard.set_with_data(&entries, move |_, sel, idx| {
            log::info!("got paste callback {}", idx);
            let idx = idx as usize;
            if idx < formats.len() {
                let item = &formats[idx];
                if item.identifier == ClipboardFormat::TEXT {
                    sel.set_text(unsafe { std::str::from_utf8_unchecked(&item.data) });
                } else {
                    let atom = Atom::intern(item.identifier);
                    let stride = 8;
                    sel.set(&atom, stride, item.data.as_slice());
                }
            }
        });
        if !success {
            log::warn!("failed to set clipboard data.");
        }
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();
        clipboard.wait_for_text().map(|s| s.to_string())
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();
        let targets = clipboard.wait_for_targets()?;
        let format_atoms = formats
            .iter()
            .map(|fmt| Atom::intern(fmt))
            .collect::<Vec<_>>();
        for atom in targets.iter() {
            if let Some(idx) = format_atoms.iter().position(|fmt| fmt == atom) {
                return Some(formats[idx]);
            }
        }
        None
    }

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();
        let atom = Atom::intern(format);
        clipboard
            .wait_for_contents(&atom)
            .map(|data| data.get_data())
    }

    pub fn available_type_names(&self) -> Vec<String> {
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();
        let targets = clipboard.wait_for_targets().unwrap_or_default();
        targets
            .iter()
            .map(|atom| unsafe { format!("{} ({})", atom.name(), atom.value()) })
            .collect()
    }
}

fn make_entries(formats: &[ClipboardFormat]) -> Vec<TargetEntry> {
    formats
        .iter()
        .enumerate()
        .map(|(i, fmt)| TargetEntry::new(fmt.identifier, TargetFlags::all(), i as u32))
        .collect()
}
