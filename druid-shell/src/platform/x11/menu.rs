use crate::hotkey::{HotKey};

pub struct Menu;

impl Menu {
    pub fn new() -> Menu {
        // TODO(x11/menus): implement Menu::new (currently a no-op)
        Menu {}
    }

    pub fn new_for_popup() -> Menu {
        // TODO(x11/menus): implement Menu::new_for_popup (currently a no-op)
        Menu {}
    }

    pub fn add_dropdown(&mut self, mut menu: Menu, text: &str, enabled: bool) {
        // TODO(x11/menus): implement Menu::add_dropdown (currently a no-op)
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: Option<&HotKey>, enabled: bool, selected: bool) {
        // TODO(x11/menus): implement Menu::add_item (currently a no-op)
    }

    pub fn add_separator(&mut self) {
        // TODO(x11/menus): implement Menu::add_separator (currently a no-op)
    }
}