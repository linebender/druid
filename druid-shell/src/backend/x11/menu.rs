// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! X11 menus implementation.

use crate::hotkey::HotKey;

pub struct Menu;

impl Menu {
    pub fn new() -> Menu {
        // TODO(x11/menus): implement Menu::new (currently a no-op)
        tracing::warn!("Menu::new is currently unimplemented for X11 backend.");
        Menu {}
    }

    pub fn new_for_popup() -> Menu {
        // TODO(x11/menus): implement Menu::new_for_popup (currently a no-op)
        tracing::warn!("Menu::new_for_popup is currently unimplemented for X11 backend.");
        Menu {}
    }

    pub fn add_dropdown(&mut self, mut _menu: Menu, _text: &str, _enabled: bool) {
        // TODO(x11/menus): implement Menu::add_dropdown (currently a no-op)
        tracing::warn!("Menu::add_dropdown is currently unimplemented for X11 backend.");
    }

    pub fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _selected: Option<bool>,
        _enabled: bool,
    ) {
        // TODO(x11/menus): implement Menu::add_item (currently a no-op)
        tracing::warn!("Menu::add_item is currently unimplemented for X11 backend.");
    }

    pub fn add_separator(&mut self) {
        // TODO(x11/menus): implement Menu::add_separator (currently a no-op)
        tracing::warn!("Menu::add_separator is currently unimplemented for X11 backend.");
    }
}
