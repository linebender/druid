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
use crate::hotkey::HotKey;
use crate::keyboard_types::{Key, Modifiers};

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
        match &self.key {
            Key::Character(t) => t,

            // from NSText.h
            Key::Enter => "\u{0003}",
            Key::Backspace => "\u{0008}",
            Key::Delete => "\u{007f}",
            // from NSEvent.h
            Key::Insert => "\u{F727}",
            Key::Home => "\u{F729}",
            Key::End => "\u{F72B}",
            Key::PageUp => "\u{F72C}",
            Key::PageDown => "\u{F72D}",
            Key::PrintScreen => "\u{F72E}",
            Key::ScrollLock => "\u{F72F}",
            Key::ArrowUp => "\u{F700}",
            Key::ArrowDown => "\u{F701}",
            Key::ArrowLeft => "\u{F702}",
            Key::ArrowRight => "\u{F703}",
            Key::F1 => "\u{F704}",
            Key::F2 => "\u{F705}",
            Key::F3 => "\u{F706}",
            Key::F4 => "\u{F707}",
            Key::F5 => "\u{F708}",
            Key::F6 => "\u{F709}",
            Key::F7 => "\u{F70A}",
            Key::F8 => "\u{F70B}",
            Key::F9 => "\u{F70C}",
            Key::F10 => "\u{F70D}",
            Key::F11 => "\u{F70E}",
            Key::F12 => "\u{F70F}",
            //Key::F13            => "\u{F710}",
            //Key::F14            => "\u{F711}",
            //Key::F15            => "\u{F712}",
            //Key::F16            => "\u{F713}",
            //Key::F17            => "\u{F714}",
            //Key::F18            => "\u{F715}",
            //Key::F19            => "\u{F716}",
            //Key::F20            => "\u{F717}",
            _ => {
                eprintln!("no key equivalent for {:?}", self);
                ""
            }
        }
    }

    fn key_modifier_mask(&self) -> NSEventModifierFlags {
        let mods: Modifiers = self.mods.into();
        let mut flags = NSEventModifierFlags::empty();
        if mods.contains(Modifiers::SHIFT) {
            flags.insert(NSEventModifierFlags::NSShiftKeyMask);
        }
        if mods.contains(Modifiers::META) {
            flags.insert(NSEventModifierFlags::NSCommandKeyMask);
        }
        if mods.contains(Modifiers::ALT) {
            flags.insert(NSEventModifierFlags::NSAlternateKeyMask);
        }
        if mods.contains(Modifiers::CONTROL) {
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
