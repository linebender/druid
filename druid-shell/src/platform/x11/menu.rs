// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! X11 menus implementation.

use crate::hotkey::HotKey;

pub struct Menu;

impl Menu {
    pub fn new() -> Menu {
        // TODO(x11/menus): implement Menu::new (currently a no-op)
        log::warn!("Menu::new is currently unimplemented for X11 platforms.");
        Menu {}
    }

    pub fn new_for_popup() -> Menu {
        // TODO(x11/menus): implement Menu::new_for_popup (currently a no-op)
        log::warn!("Menu::new_for_popup is currently unimplemented for X11 platforms.");
        Menu {}
    }

    pub fn add_dropdown(&mut self, mut _menu: Menu, _text: &str, _enabled: bool) {
        // TODO(x11/menus): implement Menu::add_dropdown (currently a no-op)
        log::warn!("Menu::add_dropdown is currently unimplemented for X11 platforms.");
    }

    pub fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _enabled: bool,
        _selected: bool,
    ) {
        // TODO(x11/menus): implement Menu::add_item (currently a no-op)
        log::warn!("Menu::add_item is currently unimplemented for X11 platforms.");
    }

    pub fn add_separator(&mut self) {
        // TODO(x11/menus): implement Menu::add_separator (currently a no-op)
        log::warn!("Menu::add_separator is currently unimplemented for X11 platforms.");
    }
}
