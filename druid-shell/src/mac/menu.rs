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
use lazy_static::lazy_static;
use objc::runtime::{Class, Object, Sel};
use objc::declare::ClassDecl;
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use cocoa::appkit::{NSMenu,NSMenuItem, NSApplication};

struct MenuItemProxyClass(*const Class);
unsafe impl Sync for MenuItemProxyClass {}

lazy_static! {
    static ref MENU_ITEM_PROXY_CLASS: MenuItemProxyClass = {
        unsafe {
            let mut decl = 
                ClassDecl::new("DruidMenuItemProxy", class!(NSObject))
                .expect("Menu class defined");
            decl.add_ivar::<u32>("menu_id");
            decl.add_method(
                sel!(trigger),
                trigger as extern "C" fn (&Object, Sel)
            );
            extern "C" fn trigger(this: &Object, _: Sel) {
                unsafe {
                    let menu_id: u32 = *this.get_ivar("menu_id");
                    println!("triggered menu item with id {}", menu_id);
                }
            }
            MenuItemProxyClass(decl.register())
        }
    };
}
pub struct Menu {
    pub menu: id,
}
fn make_basic_menu_item(id: u32, text: &str, key: &str) -> id {
    unsafe {
        NSMenuItem
            ::alloc(nil)
            .initWithTitle_action_keyEquivalent_(
                NSString::alloc(nil).init_str(text),
                sel!(trigger),
                NSString::alloc(nil).init_str(key)
            ).autorelease()
    }
}

fn make_proxy(id: u32) -> id {
    unsafe {
        let mut target: id = msg_send![MENU_ITEM_PROXY_CLASS.0, alloc];
        (*target).set_ivar("menu_id", id);
        target.autorelease()
    }
}
fn make_menu_item(id: u32, text: &str, key: &str) -> id {
    unsafe {
        let target = make_proxy(id);
        let mut menu_item = make_basic_menu_item(id, text, key);
        msg_send![menu_item, setTarget: target];
        menu_item
    }
}
impl Menu {
    pub fn new() -> Menu {
        unsafe {
            Menu { menu: NSMenu::alloc(nil) }
        }
    }

    // TODO: how do we use the text here?
    pub fn add_dropdown(&mut self, menu: Menu, text: &str) {
        unsafe {
            let menu_item = NSMenuItem::new(nil).autorelease();
            menu_item.setSubmenu_(menu.menu);
            self.menu.addItem_(menu_item);
        }
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: &str) {
        unsafe {
            let menu_item = make_menu_item(id, text, key);
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
        submenu.add_item(1, "Quit", "q");
        menu.add_dropdown(submenu, "Application");
        menu
    }
}
