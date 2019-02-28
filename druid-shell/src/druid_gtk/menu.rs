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
use gtk::Menu as GtkMenu;
use gtk::{MenuItem, MenuBar};
use gtk::{MenuExt, MenuItemExt, MenuShellExt};

pub struct Menu {
    items: Vec<MenuItem>,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            items: Vec::new(),
        }
    }

    pub fn add_dropdown(&mut self, menu: Menu, text: &str) {
        let item = MenuItem::new_with_label(text);
        item.set_submenu(&menu.into_gtk_menu());
        self.items.push(item);
    }

    pub fn add_item(&mut self, id: u32, text: &str) {
        let item = MenuItem::new_with_label(text);
        item.connect_activate(move |_widget| {
            eprintln!("Menu event emission is not yet implemented: {:?}", id);
        });
        self.items.push(item);
    }

    pub fn add_separator(&mut self) {
        eprintln!("Warning: GTK separators are not yet implemented");
    }

    pub(crate) fn into_gtk_menubar(self) -> MenuBar {
        let menu = MenuBar::new();

        for item in self.items {
            menu.append(&item);
        }

        menu
    }

    fn into_gtk_menu(self) -> GtkMenu {
        let menu = GtkMenu::new();

        for item in self.items {
            menu.append(&item);
        }

        menu
    }
}
