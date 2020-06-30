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
        log::warn!("unimplemented");
    }

    pub fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _enabled: bool,
        _selected: bool,
    ) {
        log::warn!("unimplemented");
    }

    pub fn add_separator(&mut self) {
        log::warn!("unimplemented");
    }
}
