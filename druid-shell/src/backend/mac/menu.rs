// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! macOS implementation of menus.

use cocoa::appkit::{NSEventModifierFlags, NSMenu, NSMenuItem};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{msg_send, sel, sel_impl};

use super::util::make_nsstring;
use crate::common_util::strip_access_key;
use crate::hotkey::HotKey;
use crate::keyboard::{KbKey, Modifiers};

pub struct Menu {
    pub menu: id,
}

fn make_menu_item(
    id: u32,
    text: &str,
    key: Option<&HotKey>,
    selected: Option<bool>,
    enabled: bool,
) -> id {
    let key_equivalent = key.map(HotKey::key_equivalent).unwrap_or("");
    let stripped_text = strip_access_key(text);
    unsafe {
        let item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(
                make_nsstring(&stripped_text),
                sel!(handleMenuItem:),
                make_nsstring(key_equivalent),
            )
            .autorelease();

        let () = msg_send![item, setTag: id as isize];
        if let Some(mask) = key.map(HotKey::key_modifier_mask) {
            let () = msg_send![item, setKeyEquivalentModifierMask: mask];
        }

        if !enabled {
            let () = msg_send![item, setEnabled: NO];
        }

        if let Some(true) = selected {
            let () = msg_send![item, setState: 1_isize];
        }
        item
    }
}

impl Menu {
    pub fn new() -> Menu {
        unsafe {
            let title = NSString::alloc(nil).init_str("").autorelease();
            let menu = NSMenu::alloc(nil).initWithTitle_(title).autorelease();
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
            let () = msg_send![menu_item, setTitle: title];
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
        selected: Option<bool>,
        enabled: bool,
    ) {
        let menu_item = make_menu_item(id, text, key, selected, enabled);
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
            KbKey::Character(t) => t,

            // from NSText.h
            KbKey::Enter => "\u{0003}",
            KbKey::Backspace => "\u{0008}",
            KbKey::Delete => "\u{007f}",
            // from NSEvent.h
            KbKey::Insert => "\u{F727}",
            KbKey::Home => "\u{F729}",
            KbKey::End => "\u{F72B}",
            KbKey::PageUp => "\u{F72C}",
            KbKey::PageDown => "\u{F72D}",
            KbKey::PrintScreen => "\u{F72E}",
            KbKey::ScrollLock => "\u{F72F}",
            KbKey::ArrowUp => "\u{F700}",
            KbKey::ArrowDown => "\u{F701}",
            KbKey::ArrowLeft => "\u{F702}",
            KbKey::ArrowRight => "\u{F703}",
            KbKey::F1 => "\u{F704}",
            KbKey::F2 => "\u{F705}",
            KbKey::F3 => "\u{F706}",
            KbKey::F4 => "\u{F707}",
            KbKey::F5 => "\u{F708}",
            KbKey::F6 => "\u{F709}",
            KbKey::F7 => "\u{F70A}",
            KbKey::F8 => "\u{F70B}",
            KbKey::F9 => "\u{F70C}",
            KbKey::F10 => "\u{F70D}",
            KbKey::F11 => "\u{F70E}",
            KbKey::F12 => "\u{F70F}",
            KbKey::F13 => "\u{F710}",
            KbKey::F14 => "\u{F711}",
            KbKey::F15 => "\u{F712}",
            KbKey::F16 => "\u{F713}",
            KbKey::F17 => "\u{F714}",
            KbKey::F18 => "\u{F715}",
            KbKey::F19 => "\u{F716}",
            KbKey::F20 => "\u{F717}",
            _ => {
                eprintln!("no key equivalent for {self:?}");
                ""
            }
        }
    }

    fn key_modifier_mask(&self) -> NSEventModifierFlags {
        let mods: Modifiers = self.mods.into();
        let mut flags = NSEventModifierFlags::empty();
        if mods.shift() {
            flags.insert(NSEventModifierFlags::NSShiftKeyMask);
        }
        if mods.meta() {
            flags.insert(NSEventModifierFlags::NSCommandKeyMask);
        }
        if mods.alt() {
            flags.insert(NSEventModifierFlags::NSAlternateKeyMask);
        }
        if mods.ctrl() {
            flags.insert(NSEventModifierFlags::NSControlKeyMask);
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn strip_access() {
        assert_eq!(strip_access_key("&Exit").as_str(), "Exit");
        assert_eq!(strip_access_key("&&Exit").as_str(), "&Exit");
        assert_eq!(strip_access_key("E&&xit").as_str(), "E&xit");
        assert_eq!(strip_access_key("E&xit").as_str(), "Exit");
    }
}
