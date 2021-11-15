use std::path::PathBuf;

use crate::backend::dnd as backend;
use crate::{Counter, FormatId};
use bitflags::bitflags;

use piet_common::ImageBuf;

#[derive(Debug)]
pub enum DragDropAction {
    Copy,
    Move,
    Link,
}

bitflags! {
    struct DragDropActions: u32 {
        const COPY = 1 << 0;
        const MOVE = 1 << 1;
        const LINK = 1 << 2;
    }
}

#[derive(Debug)]
pub struct DragData(backend::DragData);

#[derive(Clone)]
pub struct DropContext(pub(crate) backend::DragDropContext);

/// A unique identifier for a drag drop session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DragDropToken(u64);

impl DragDropToken {
    /// Create a new, unique DragDropId
    pub(crate) fn next() -> DragDropToken {
        static COUNTER: Counter = Counter::new();
        DragDropToken(COUNTER.next())
    }
}

impl DragData {
    pub fn new(actions: DragDropActions) -> Self {
        DragData(backend::DragData::new(actions))
    }

    /// Add a data format
    pub fn add(&mut self, format: FormatId, data: Vec<u8>) {
        self.0.add(format, data)
    }

    pub fn cursor_image(&mut self, image: ImageBuf) {
        self.0.cursor_image(image)
    }

    pub fn files(&mut self, files: Vec<PathBuf>) {
        self.0.files(files)
    }
}

impl DropContext {
    pub fn cancel(&self) {
        self.0.cancel()
    }

    pub fn allowed_actions(&self) -> DragDropActions {
        self.0.allowed_actions()
    }

    pub fn set_action(&self, action: DragDropAction) {
        self.0.set_action(action)
    }

    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        self.0.get_format(format)
    }

    pub fn files(&self) -> Option<Vec<PathBuf>> {
        self.0.files()
    }

    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        self.0.preferred_format(formats)
    }

    pub fn token(&self) -> DragDropToken {
        self.0.token()
    }
}
