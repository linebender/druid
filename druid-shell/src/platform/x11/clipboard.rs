use crate::clipboard::{ClipboardFormat, FormatId};

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    pub fn put_string(&mut self, s: impl AsRef<str>) {
        unimplemented!(); // TODO
    }

    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        unimplemented!(); // TODO
    }

    pub fn get_string(&self) -> Option<String> {
        unimplemented!(); // TODO
    }

    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        unimplemented!(); // TODO
    }

    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        unimplemented!(); // TODO
    }

    pub fn available_type_names(&self) -> Vec<String> {
        unimplemented!(); // TODO
    }
}