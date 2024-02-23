// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Safe wrapper for menus.

use crate::hotkey::HotKey;

/// A menu object, which can be either a top-level menubar or a
/// submenu.
pub struct Menu;

impl Drop for Menu {
    fn drop(&mut self) {
        // TODO
    }
}

impl Menu {
    pub fn new() -> Menu {
        Menu
    }

    pub fn new_for_popup() -> Menu {
        Menu
    }

    pub fn add_dropdown(&mut self, _menu: Menu, _text: &str, _enabled: bool) {
        tracing::warn!("unimplemented");
    }

    pub fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _selected: Option<bool>,
        _enabled: bool,
    ) {
        tracing::warn!("unimplemented");
    }

    pub fn add_separator(&mut self) {
        tracing::warn!("unimplemented");
    }
}
