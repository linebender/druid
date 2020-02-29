use crate::hotkey::{HotKey};

pub struct Menu;

impl Menu {
    pub fn new() -> Menu {
        // TODO: currently a no-op.
        Menu {}
    }

    pub fn new_for_popup() -> Menu {
        // TODO: currently a no-op.
        Menu {}
    }

    pub fn add_dropdown(&mut self, mut menu: Menu, text: &str, enabled: bool) {
        // TODO: currently a no-op.
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: Option<&HotKey>, enabled: bool, selected: bool) {
        // TODO: currently a no-op.
    }

    pub fn add_separator(&mut self) {
        // TODO: currently a no-op.
    }
}