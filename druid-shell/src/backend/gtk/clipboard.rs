// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactions with the system pasteboard on GTK+.

use gtk::gdk::Atom;
use gtk::{TargetEntry, TargetFlags};

use crate::clipboard::{ClipboardFormat, FormatId};

const CLIPBOARD_TARGETS: [&str; 5] = [
    "UTF8_STRING",
    "TEXT",
    "STRING",
    "text/plain;charset=utf-8",
    "text/plain",
];

/// The system clipboard.
#[derive(Clone)]
pub struct Clipboard {
    pub(crate) selection: Atom,
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.selection {
            gtk::gdk::SELECTION_PRIMARY => "Primary",
            gtk::gdk::SELECTION_CLIPBOARD => "Clipboard",
            _ => "(other)",
        };
        f.debug_tuple("Clipboard").field(&name).finish()
    }
}

impl Clipboard {
    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, string: impl AsRef<str>) {
        let string = string.as_ref().to_string();

        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = gtk::Clipboard::for_display(&display, &self.selection);

        let targets: Vec<TargetEntry> = CLIPBOARD_TARGETS
            .iter()
            .enumerate()
            .map(|(i, target)| TargetEntry::new(target, TargetFlags::all(), i as u32))
            .collect();

        clipboard.set_with_data(&targets, move |_, selection, _| {
            const STRIDE_BITS: i32 = 8;
            selection.set(&selection.target(), STRIDE_BITS, string.as_bytes());
        });
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        let entries = make_entries(formats);
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = gtk::Clipboard::for_display(&display, &self.selection);

        // this is gross: we need to reclone all the data in formats in order
        // to move it into the closure. :/
        let formats = formats.to_owned();
        let success = clipboard.set_with_data(&entries, move |_, sel, idx| {
            tracing::info!("got paste callback {}", idx);
            let idx = idx as usize;
            if idx < formats.len() {
                let item = &formats[idx];
                if let (ClipboardFormat::TEXT, Ok(data)) =
                    (item.identifier, std::str::from_utf8(&item.data))
                {
                    sel.set_text(data);
                } else {
                    let atom = Atom::intern(item.identifier);
                    let stride = 8;
                    sel.set(&atom, stride, item.data.as_slice());
                }
            }
        });
        if !success {
            tracing::warn!("failed to set clipboard data.");
        }
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = gtk::Clipboard::for_display(&display, &self.selection);

        for target in &CLIPBOARD_TARGETS {
            let atom = Atom::intern(target);
            if let Some(selection) = clipboard.wait_for_contents(&atom) {
                return String::from_utf8(selection.data()).ok();
            }
        }

        None
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = gtk::Clipboard::for_display(&display, &self.selection);

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
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = gtk::Clipboard::for_display(&display, &self.selection);

        let atom = Atom::intern(format);
        clipboard.wait_for_contents(&atom).map(|data| data.data())
    }

    pub fn available_type_names(&self) -> Vec<String> {
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = gtk::Clipboard::for_display(&display, &self.selection);

        let targets = clipboard.wait_for_targets().unwrap_or_default();
        targets
            .iter()
            .map(|atom| format!("{} ({})", atom.name(), atom.value()))
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
