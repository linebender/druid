use crate::hotkey::{HotKey};

pub struct Menu;

impl Menu {
    pub fn new() -> Menu {
        unimplemented!(); // TODO
    }

    pub fn new_for_popup() -> Menu {
        unimplemented!(); // TODO
    }

    pub fn add_dropdown(&mut self, mut menu: Menu, text: &str, enabled: bool) {
        unimplemented!(); // TODO
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: Option<&HotKey>, enabled: bool, selected: bool) {
        unimplemented!(); // TODO
    }

    pub fn add_separator(&mut self) {
        unimplemented!(); // TODO
    }
}