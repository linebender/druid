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

use gdk::ModifierType;
use gtkrs::AccelGroup;
use gtkrs::GtkMenuExt;
use gtkrs::Menu as GtkMenu;
use gtkrs::MenuBar as GtkMenuBar;
use gtkrs::MenuItem as GtkMenuItem;
use gtkrs::{GtkMenuItemExt, MenuShellExt, SeparatorMenuItemBuilder, WidgetExt};

use crate::gtk::WinCtxImpl;
use crate::keycodes::{KeySpec, MenuKey};
use crate::keycodes::{Modifiers, M_ALT, M_CTRL, M_META, M_SHIFT};
use crate::platform::WindowHandle;

#[derive(Default)]
pub struct Menu {
    items: Vec<MenuItem>,
}

enum MenuItem {
    Entry(String, u32, MenuKey),
    SubMenu(String, Menu),
    Separator,
}

impl Menu {
    pub fn new() -> Menu {
        Menu { items: Vec::new() }
    }

    pub fn add_dropdown(&mut self, menu: Menu, text: &str) {
        self.items
            .push(MenuItem::SubMenu(strip_access_key(text), menu));
    }

    pub fn add_item(&mut self, id: u32, text: &str, key: impl Into<MenuKey>) {
        // TODO: handle accelerator shortcuts by parsing `text`
        self.items
            .push(MenuItem::Entry(strip_access_key(text), id, key.into()));
    }

    pub fn add_separator(&mut self) {
        self.items.push(MenuItem::Separator)
    }

    fn append_items_to_menu<M: gtkrs::prelude::IsA<gtkrs::MenuShell>>(
        self,
        menu: &mut M,
        handle: &WindowHandle,
        accel_group: &AccelGroup,
    ) {
        for item in self.items {
            match item {
                MenuItem::Entry(name, id, key) => {
                    let item = GtkMenuItem::new_with_label(&name);

                    register_accelerator(&item, accel_group, key);

                    let handle = handle.clone();
                    item.connect_activate(move |_| {
                        let mut ctx = WinCtxImpl::from(&handle);

                        if let Some(state) = handle.state.upgrade() {
                            state.handler.borrow_mut().command(id, &mut ctx);
                        }
                    });

                    menu.append(&item);
                }
                MenuItem::SubMenu(name, submenu) => {
                    let item = GtkMenuItem::new_with_label(&name);
                    item.set_submenu(Some(&submenu.into_gtk_menu(handle, accel_group)));

                    menu.append(&item);
                }
                MenuItem::Separator => menu.append(&SeparatorMenuItemBuilder::new().build()),
            }
        }
    }

    pub(crate) fn into_gtk_menubar(
        self,
        handle: &WindowHandle,
        accel_group: &AccelGroup,
    ) -> GtkMenuBar {
        let mut menu = GtkMenuBar::new();

        self.append_items_to_menu(&mut menu, handle, accel_group);

        menu
    }

    fn into_gtk_menu(self, handle: &WindowHandle, accel_group: &AccelGroup) -> GtkMenu {
        let mut menu = GtkMenu::new();
        menu.set_accel_group(Some(accel_group));

        self.append_items_to_menu(&mut menu, handle, accel_group);

        menu
    }
}

fn register_accelerator(item: &GtkMenuItem, accel_group: &AccelGroup, menu_key: MenuKey) {
    if let KeySpec::Char(c) = menu_key.key {
        item.add_accelerator(
            "activate",
            accel_group,
            gdk::unicode_to_keyval(c as u32),
            modifiers_to_gdk_modifier_type(menu_key.modifiers),
            gtk::AccelFlags::VISIBLE,
        );
    }
}

fn modifiers_to_gdk_modifier_type(modifiers: Modifiers) -> gdk::ModifierType {
    let mut result = ModifierType::empty();

    result.set(ModifierType::MOD1_MASK, modifiers & M_ALT == M_ALT);
    result.set(ModifierType::CONTROL_MASK, modifiers & M_CTRL == M_CTRL);
    result.set(ModifierType::SHIFT_MASK, modifiers & M_SHIFT == M_SHIFT);
    result.set(ModifierType::META_MASK, modifiers & M_META == M_META);

    result
}

/// Strip the access keys from the menu string.
///
/// Changes "E&xit" to "Exit". Actual ampersands are escaped as "&&".
fn strip_access_key(raw_menu_text: &str) -> String {
    // TODO this is copied from mac/menu.rs maybe this should be moved somewhere common?
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
