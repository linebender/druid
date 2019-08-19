// Copyright 2018 The xi-editor Authors.
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

use std::mem;
use std::ptr::null;

use winapi::shared::basetsd::*;
use winapi::shared::windef::*;
use winapi::um::winuser::*;

use crate::hotkey::{HotKey, KeyCompare};
use crate::util::ToWide;

/// A menu object, which can be either a top-level menubar or a
/// submenu.
pub struct Menu {
    hmenu: HMENU,
}

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.hmenu);
        }
    }
}

impl Menu {
    pub fn new() -> Menu {
        unsafe {
            let hmenu = CreateMenu();
            Menu { hmenu }
        }
    }

    pub fn into_hmenu(self) -> HMENU {
        let hmenu = self.hmenu;
        mem::forget(self);
        hmenu
    }

    /// Add a dropdown menu. This takes the menu by ownership, but we'll
    /// probably want to change that so we can manipulate it later.
    ///
    /// The `text` field has all the fun behavior of winapi CreateMenu.
    pub fn add_dropdown(&mut self, menu: Menu, text: &str, enabled: bool) {
        unsafe {
            let mut flags = MF_POPUP;
            if !enabled {
                flags &= MF_GRAYED;
            }
            AppendMenuW(
                self.hmenu,
                flags,
                menu.into_hmenu() as UINT_PTR,
                text.to_wide().as_ptr(),
            );
        }
    }

    /// Add an item to the menu.
    pub fn add_item(
        &mut self,
        id: u32,
        text: &str,
        _key: Option<&HotKey>,
        enabled: bool,
        selected: bool,
    ) {
        // TODO: actually wire up accelerators for key.
        unsafe {
            let mut flags = MF_STRING;
            if !enabled {
                flags &= MF_GRAYED;
            }
            if !selected {
                flags &= MF_CHECKED;
            }
            AppendMenuW(self.hmenu, flags, id as UINT_PTR, text.to_wide().as_ptr());
        }
    }

    /// Add a separator to the menu.
    pub fn add_separator(&mut self) {
        unsafe {
            AppendMenuW(self.hmenu, MF_SEPARATOR, 0, null());
        }
    }
}
