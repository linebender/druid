// Copyright 2019 The xi-editor Authors.
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

//! macOS implementation of menus.

use cocoa::appkit::{NSEventModifierFlags, NSMenu, NSMenuItem};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::NSAutoreleasePool;
use objc::{msg_send, sel, sel_impl};

use super::util::make_nsstring;
use crate::common_util::strip_access_key;
use crate::hotkey::{HotKey, KeyCompare};
use crate::keyboard::KeyModifiers;
use crate::keycodes::KeyCode;

pub struct Menu {
    pub menu: id,
}

fn make_menu_item(id: u32, text: &str, key: Option<&HotKey>, enabled: bool, selected: bool) -> id {
    let key_equivalent = key.map(HotKey::key_equivalent).unwrap_or("");
    let stripped_text = strip_access_key(text);
    unsafe {
        let item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(
                make_nsstring(&stripped_text),
                sel!(handleMenuItem:),
                make_nsstring(&key_equivalent),
            )
            .autorelease();

        let () = msg_send![item, setTag: id as isize];
        if let Some(mask) = key.map(HotKey::key_modifier_mask) {
            let () = msg_send![item, setKeyEquivalentModifierMask: mask];
        }

        if !enabled {
            let () = msg_send![item, setEnabled: NO];
        }

        if selected {
            let () = msg_send![item, setState: 1_isize];
        }
        item
    }
}

impl Menu {
    pub fn new() -> Menu {
        unsafe {
            let menu = NSMenu::alloc(nil).autorelease();
            let () = msg_send![menu, setAutoenablesItems: NO];
            Menu { menu }
        }
    }

    pub fn new_for_popup() -> Menu {
        // mac doesn't distinguish between application and context menu types.
        Menu::new()
    }

    pub fn add_dropdown(&mut self, menu: Menu, text: &str, enabled: bool) {
        unsafe {
            let menu_item = NSMenuItem::alloc(nil).autorelease();
            let title = make_nsstring(&strip_access_key(text));
            let () = msg_send![menu.menu, setTitle: title];
            if !enabled {
                let () = msg_send![menu_item, setEnabled: NO];
            }
            menu_item.setSubmenu_(menu.menu);
            self.menu.addItem_(menu_item);
        }
    }

    pub fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        enabled: bool,
        selected: bool,
    ) {
        let menu_item = make_menu_item(id, text, key, enabled, selected);
        unsafe {
            self.menu.addItem_(menu_item);
        }
    }

    pub fn add_separator(&mut self) {
        unsafe {
            let sep = id::separatorItem(self.menu);
            self.menu.addItem_(sep);
        }
    }
}

impl HotKey {
    /// Return the string value of this hotkey, for use with Cocoa `NSResponder`
    /// objects.
    ///
    /// Returns the empty string if no key equivalent is known.
    fn key_equivalent(&self) -> &str {
        match self.key {
            KeyCompare::Text(t) => t,

            // from NSText.h
            KeyCompare::Code(KeyCode::Return) => "\u{0003}",
            KeyCompare::Code(KeyCode::Backspace) => "\u{0008}",
            KeyCompare::Code(KeyCode::Delete) => "\u{007f}",
            // from NSEvent.h
            KeyCompare::Code(KeyCode::Insert) => "\u{F727}",
            KeyCompare::Code(KeyCode::Home) => "\u{F729}",
            KeyCompare::Code(KeyCode::End) => "\u{F72B}",
            KeyCompare::Code(KeyCode::PageUp) => "\u{F72C}",
            KeyCompare::Code(KeyCode::PageDown) => "\u{F72D}",
            KeyCompare::Code(KeyCode::PrintScreen) => "\u{F72E}",
            KeyCompare::Code(KeyCode::ScrollLock) => "\u{F72F}",
            KeyCompare::Code(KeyCode::ArrowUp) => "\u{F700}",
            KeyCompare::Code(KeyCode::ArrowDown) => "\u{F701}",
            KeyCompare::Code(KeyCode::ArrowLeft) => "\u{F702}",
            KeyCompare::Code(KeyCode::ArrowRight) => "\u{F703}",
            KeyCompare::Code(KeyCode::F1) => "\u{F704}",
            KeyCompare::Code(KeyCode::F2) => "\u{F705}",
            KeyCompare::Code(KeyCode::F3) => "\u{F706}",
            KeyCompare::Code(KeyCode::F4) => "\u{F707}",
            KeyCompare::Code(KeyCode::F5) => "\u{F708}",
            KeyCompare::Code(KeyCode::F6) => "\u{F709}",
            KeyCompare::Code(KeyCode::F7) => "\u{F70A}",
            KeyCompare::Code(KeyCode::F8) => "\u{F70B}",
            KeyCompare::Code(KeyCode::F9) => "\u{F70C}",
            KeyCompare::Code(KeyCode::F10) => "\u{F70D}",
            //KeyCompare::Code(KeyCode::F11)            => "\u{F70E}",
            //KeyCompare::Code(KeyCode::F12)            => "\u{F70F}",
            //KeyCompare::Code(KeyCode::F13)            => "\u{F710}",
            //KeyCompare::Code(KeyCode::F14)            => "\u{F711}",
            //KeyCompare::Code(KeyCode::F15)            => "\u{F712}",
            //KeyCompare::Code(KeyCode::F16)            => "\u{F713}",
            //KeyCompare::Code(KeyCode::F17)            => "\u{F714}",
            //KeyCompare::Code(KeyCode::F18)            => "\u{F715}",
            //KeyCompare::Code(KeyCode::F19)            => "\u{F716}",
            //KeyCompare::Code(KeyCode::F20)            => "\u{F717}",
            KeyCompare::Code(other) => {
                eprintln!("no key equivalent for {:?}", other);
                ""
            }
        }
    }

    fn key_modifier_mask(&self) -> NSEventModifierFlags {
        let mods: KeyModifiers = self.mods.into();
        let mut flags = NSEventModifierFlags::empty();
        if mods.shift {
            flags.insert(NSEventModifierFlags::NSShiftKeyMask);
        }
        if mods.meta {
            flags.insert(NSEventModifierFlags::NSCommandKeyMask);
        }
        if mods.alt {
            flags.insert(NSEventModifierFlags::NSAlternateKeyMask);
        }
        if mods.ctrl {
            flags.insert(NSEventModifierFlags::NSControlKeyMask);
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strip_access() {
        assert_eq!(strip_access_key("&Exit").as_str(), "Exit");
        assert_eq!(strip_access_key("&&Exit").as_str(), "&Exit");
        assert_eq!(strip_access_key("E&&xit").as_str(), "E&xit");
        assert_eq!(strip_access_key("E&xit").as_str(), "Exit");
    }
}
