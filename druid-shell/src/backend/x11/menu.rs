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
use std::any::Any;

use crate::hotkey::HotKey;
use crate::menu::MenuBackend;

impl MenuBackend for Menu {
    fn add_dropdown(&mut self, menu: crate::menu::Menu, text: &str, enabled: bool) {
        if let Some(menu) = menu.0.as_any().downcast_ref::<Menu>() {
            self.add_dropdown(*menu, text, enabled)
        }
    }

    fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        enabled: bool,
        selected: bool,
    ) {
        self.add_item(id, text, key, enabled, selected)
    }

    fn add_separator(&mut self) {
        self.add_separator()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> String {
        "x11".into()
    }
}

#[derive(Copy, Clone)]
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

    fn add_dropdown(&mut self, mut _menu: Menu, _text: &str, _enabled: bool) {
        // TODO(x11/menus): implement Menu::add_dropdown (currently a no-op)
        tracing::warn!("Menu::add_dropdown is currently unimplemented for X11 backend.");
    }

    fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _enabled: bool,
        _selected: bool,
    ) {
        // TODO(x11/menus): implement Menu::add_item (currently a no-op)
        tracing::warn!("Menu::add_item is currently unimplemented for X11 backend.");
    }

    fn add_separator(&mut self) {
        // TODO(x11/menus): implement Menu::add_separator (currently a no-op)
        tracing::warn!("Menu::add_separator is currently unimplemented for X11 backend.");
    }
}
