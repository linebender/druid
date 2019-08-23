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
use crate::keycodes::{KeySpec, MenuKey};
use crate::util::make_nsstring;
use cocoa::appkit::{NSMenu, NSMenuItem};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::NSAutoreleasePool;

pub struct Menu {
    pub menu: id,
}

/// Make a string in the syntax expected as the keyEquivalent argument to
/// NSMenuItem initWithTitle:action:keyEquivalent:
fn make_key_equivalent(key: impl Into<MenuKey>) -> String {
    // TODO: handle modifiers and other fun things
    let key = key.into();
    match key.key {
        KeySpec::Char(c) => c.to_string(),
        KeySpec::None => "".to_string(),
    }
}

/// Strip the access keys from the menu strong.
///
/// Changes "E&xit" to "Exit". Actual ampersands are escaped as "&&".
fn strip_access_key(raw_menu_text: &str) -> String {
    let mut saw_ampersand = false;
    let mut result = String::new();
    for c in raw_menu_text.chars() {
        if c == '&' {
            if saw_ampersand {
                result.push(c);
            }
            saw_ampersand = !saw_ampersand;
        } else {
            result.push(c);
            saw_ampersand = false;
        }
    }
    result
}

fn make_menu_item(id: u32, text: &str, key: impl Into<MenuKey>) -> id {
    let key_equivalent = make_key_equivalent(key);
    let stripped_text = strip_access_key(text);
    unsafe {
        let item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(
                make_nsstring(&stripped_text),
                sel!(handleMenuItem:),
                make_nsstring(&key_equivalent),
            )
            .autorelease();
        msg_send![item, setTag: id as isize];
        item
    }
}

impl Menu {
    pub fn new() -> Menu {
        unsafe {
            let menu = NSMenu::alloc(nil);
            msg_send![menu, setAutoenablesItems: NO];
            Menu { menu }
        }
    }

    // TODO: how do we use the text here?
    pub fn add_dropdown(&mut self, menu: Menu, _text: &str) {
        unsafe {
            let menu_item = NSMenuItem::new(nil).autorelease();
            menu_item.setSubmenu_(menu.menu);
            self.menu.addItem_(menu_item);
        }
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: impl Into<MenuKey>) {
        let menu_item = make_menu_item(id, text, key);
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

impl Default for Menu {
    fn default() -> Menu {
        // The top level menu is just to contain the menus
        let mut menu = Menu::new();
        // this one is our actual menu
        let mut submenu = Menu::new();
        submenu.add_item(1, "Quit", 'q');
        menu.add_dropdown(submenu, "Application");
        menu
    }
}
