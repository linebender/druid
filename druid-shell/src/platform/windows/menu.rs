// Copyright 2018 The Druid Authors.
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

use std::collections::HashMap;
use std::mem;
use std::ptr::null;

use winapi::shared::basetsd::*;
use winapi::shared::windef::*;
use winapi::um::winuser::*;

use super::util::ToWide;
use crate::hotkey::HotKey;
use crate::keyboard::{KbKey, Modifiers};

/// A menu object, which can be either a top-level menubar or a
/// submenu.
pub struct Menu {
    hmenu: HMENU,
    accels: HashMap<u32, ACCEL>,
}

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.hmenu);
        }
    }
}

impl Menu {
    /// Create a new menu for a window.
    pub fn new() -> Menu {
        unsafe {
            let hmenu = CreateMenu();
            Menu {
                hmenu,
                accels: HashMap::default(),
            }
        }
    }

    /// Create a new popup (context / right-click) menu.
    pub fn new_for_popup() -> Menu {
        unsafe {
            let hmenu = CreatePopupMenu();
            Menu {
                hmenu,
                accels: HashMap::default(),
            }
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
    pub fn add_dropdown(&mut self, mut menu: Menu, text: &str, enabled: bool) {
        let child_accels = std::mem::take(&mut menu.accels);
        self.accels.extend(child_accels);

        unsafe {
            let mut flags = MF_POPUP;
            if !enabled {
                flags |= MF_GRAYED;
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
        key: Option<&HotKey>,
        enabled: bool,
        selected: bool,
    ) {
        let mut anno_text = text.to_string();
        if let Some(key) = key {
            anno_text.push('\t');
            format_hotkey(&key, &mut anno_text);
        }
        unsafe {
            let mut flags = MF_STRING;
            if !enabled {
                flags |= MF_GRAYED;
            }
            if selected {
                flags |= MF_CHECKED;
            }
            AppendMenuW(
                self.hmenu,
                flags,
                id as UINT_PTR,
                anno_text.to_wide().as_ptr(),
            );
        }

        if let Some(key) = key {
            if let Some(accel) = convert_hotkey(id, key) {
                self.accels.insert(id, accel);
            }
        }
    }

    /// Add a separator to the menu.
    pub fn add_separator(&mut self) {
        unsafe {
            AppendMenuW(self.hmenu, MF_SEPARATOR, 0, null());
        }
    }

    /// Get the accels table
    pub fn accels(&self) -> Option<Vec<ACCEL>> {
        if self.accels.is_empty() {
            return None;
        }
        Some(self.accels.values().cloned().collect())
    }
}

/// Convert a hotkey to an accelerator.
///
/// Note that this conversion is dependent on the keyboard map.
/// Therefore, when the keyboard map changes (WM_INPUTLANGCHANGE),
/// we should be rebuilding the accelerator map.
fn convert_hotkey(id: u32, key: &HotKey) -> Option<ACCEL> {
    let mut virt_key = FVIRTKEY;
    let key_mods: Modifiers = key.mods.into();
    if key_mods.ctrl() {
        virt_key |= FCONTROL;
    }
    if key_mods.alt() {
        virt_key |= FALT;
    }
    if key_mods.shift() {
        virt_key |= FSHIFT;
    }

    let raw_key = if let Some(vk_code) = super::keyboard::key_to_vk(&key.key) {
        let mod_code = vk_code >> 8;
        if mod_code & 0x1 != 0 {
            virt_key |= FSHIFT;
        }
        if mod_code & 0x02 != 0 {
            virt_key |= FCONTROL;
        }
        if mod_code & 0x04 != 0 {
            virt_key |= FALT;
        }
        vk_code & 0x00ff
    } else {
        log::error!("Failed to convert key {:?} into virtual key code", key.key);
        return None;
    };

    Some(ACCEL {
        fVirt: virt_key,
        key: raw_key as u16,
        cmd: id as u16,
    })
}

/// Format the hotkey in a Windows-native way.
fn format_hotkey(key: &HotKey, s: &mut String) {
    let key_mods: Modifiers = key.mods.into();
    if key_mods.ctrl() {
        s.push_str("Ctrl+");
    }
    if key_mods.shift() {
        s.push_str("Shift+");
    }
    if key_mods.alt() {
        s.push_str("Alt+");
    }
    if key_mods.meta() {
        s.push_str("Windows+");
    }
    match &key.key {
        KbKey::Character(c) => match c.as_str() {
            "+" => s.push_str("Plus"),
            "-" => s.push_str("Minus"),
            " " => s.push_str("Space"),
            _ => s.extend(c.chars().flat_map(|c| c.to_uppercase())),
        },
        KbKey::Escape => s.push_str("Esc"),
        KbKey::Delete => s.push_str("Del"),
        KbKey::Insert => s.push_str("Ins"),
        KbKey::PageUp => s.push_str("PgUp"),
        KbKey::PageDown => s.push_str("PgDn"),
        // These names match LibreOffice.
        KbKey::ArrowLeft => s.push_str("Left"),
        KbKey::ArrowRight => s.push_str("Right"),
        KbKey::ArrowUp => s.push_str("Up"),
        KbKey::ArrowDown => s.push_str("Down"),
        _ => s.push_str(&format!("{:?}", key.key)),
    }
}
