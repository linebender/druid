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

//! GTK implementation of menus.

use crate::window;

use std::cell::RefCell;
use std::sync::Arc;

use gtk::Menu as GtkMenu;
use gtk::MenuBar as GtkMenuBar;
use gtk::MenuItem as GtkMenuItem;
use gtk::{GtkMenuItemExt, MenuShellExt};

use crate::platform::WindowHandle;

use crate::keycodes::MenuKey;

use crate::druid_gtk::WinCtxImpl;
use crate::window::Text;

pub struct Menu {
    items: Vec<MenuItem>,
}

enum MenuItem {
    Entry(String, u32),
    SubMenu(String, Menu),
}

impl MenuItem {
    /// Get the name of this menu item
    fn name(&self) -> &str {
        match self {
            MenuItem::Entry(name, _) | MenuItem::SubMenu(name, _) => name,
        }
    }

    fn into_gtk_menu_item(
        self,
        handler: Arc<RefCell<Box<dyn window::WinHandler>>>,
        handle: &WindowHandle,
    ) -> GtkMenuItem {
        match self {
            MenuItem::Entry(name, id) => {
                let handle = handle.clone();
                let item = GtkMenuItem::new_with_label(&name);
                item.connect_activate(move |_| {
                    let mut ctx = WinCtxImpl {
                        handle: &handle,
                        window: None,
                        text: Text::new(),
                    };

                    if let Ok(mut handler) = handler.try_borrow_mut() {
                        handler.command(id, &mut ctx);
                    }
                });

                item
            }
            MenuItem::SubMenu(name, submenu) => {
                let item = GtkMenuItem::new_with_label(&name);
                item.set_submenu(Some(&submenu.into_gtk_menu(handler, handle)));

                item
            }
        }
    }
}

impl Menu {
    pub fn new() -> Menu {
        Menu { items: Vec::new() }
    }

    pub fn add_dropdown(&mut self, menu: Menu, text: &str) {
        self.items.push(MenuItem::SubMenu(text.into(), menu));
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: impl Into<MenuKey>) {
        // TODO: handle accelerator shortcuts by parsing `text`
        self.items.push(MenuItem::Entry(text.into(), id));
    }

    pub fn add_separator(&mut self) {
        eprintln!("Warning: GTK separators are not yet implemented");
    }

    pub(crate) fn into_gtk_menubar(
        self,
        handler: Arc<RefCell<Box<dyn window::WinHandler>>>,
        handle: &WindowHandle,
    ) -> GtkMenuBar {
        let menu = GtkMenuBar::new();

        for item in self.items {
            menu.append(&item.into_gtk_menu_item(handler.clone(), handle));
        }

        menu
    }

    fn into_gtk_menu(
        self,
        handler: Arc<RefCell<Box<dyn window::WinHandler>>>,
        handle: &WindowHandle,
    ) -> GtkMenu {
        let menu = GtkMenu::new();

        for item in self.items {
            menu.append(&item.into_gtk_menu_item(handler.clone(), handle));
        }

        menu
    }
}
