use std::path::PathBuf;

use crate::backend::dnd as backend;
use crate::{Counter, FormatId, Modifiers};

use kurbo::Point;
use piet_common::ImageBuf;

#[derive(Debug)]
pub enum DragDropAction {
    Copy,
    Move,
}

#[derive(Debug)]
pub struct DragData(backend::DragData);

#[derive(Debug, Clone)]
pub struct DropEvent {
    pub modifiers: Modifiers,
    pub position: Point,
}

#[derive(Clone)]
pub struct DropContext(pub(crate) backend::DragDropContext);

/// A unique identifier for a drag drop session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DragDropId(u64);

impl DragDropId {
    /// Create a new, unique DragDropId
    pub(crate) fn next() -> DragDropId {
        static COUNTER: Counter = Counter::new();
        DragDropId(COUNTER.next())
    }
}

impl DragData {
    pub fn new() -> Self {
        DragData(backend::DragData::new())
    }

    /// Add a data format
    pub fn add(&mut self, format: FormatId, data: Vec<u8>) {
        self.0.add(format, data)
    }

    /// default: true
    pub fn copyable(&mut self, allowed: bool) {
        self.0.copyable(allowed)
    }

    /// default: false
    pub fn movable(&mut self, allowed: bool) {
        self.0.movable(allowed)
    }

    pub fn cursor_image(&mut self, image: ImageBuf) {
        self.0.cursor_image(image)
    }

    pub fn files(&mut self, files: Vec<PathBuf>) {
        self.0.files(files)
    }
}

impl DropContext {
    pub fn deny(&self) {
        self.0.deny()
    }

    pub fn action(&self) -> DragDropAction {
        self.0.action()
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

    pub fn id(&self) -> DragDropId {
        self.0.id()
    }
}
