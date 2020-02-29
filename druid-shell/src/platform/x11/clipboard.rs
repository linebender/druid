use crate::clipboard::{ClipboardFormat, FormatId};

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    pub fn put_string(&mut self, s: impl AsRef<str>) {
        // TODO(x11/clipboard): implement Clipboard::put_string
        unimplemented!();
    }

    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        // TODO(x11/clipboard): implement Clipboard::put_formats
        unimplemented!();
    }

    pub fn get_string(&self) -> Option<String> {
        // TODO(x11/clipboard): implement Clipboard::get_string
        unimplemented!();
    }

    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        // TODO(x11/clipboard): implement Clipboard::preferred_format
        unimplemented!();
    }

    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        // TODO(x11/clipboard): implement Clipboard::get_format
        unimplemented!();
    }

    pub fn available_type_names(&self) -> Vec<String> {
        // TODO(x11/clipboard): implement Clipboard::available_type_names
        unimplemented!();
    }
}