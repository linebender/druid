// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! GTK implementation of menus.

use gtk::gdk::ModifierType;
use gtk::{
    AccelGroup, CheckMenuItem, Menu as GtkMenu, MenuBar as GtkMenuBar, MenuItem as GtkMenuItem,
};
use gtk_rs::SeparatorMenuItem;

use gtk::prelude::{GtkMenuExt, GtkMenuItemExt, MenuShellExt, WidgetExt};

use super::keycodes;
use super::window::WindowHandle;
use crate::common_util::strip_access_key;
use crate::hotkey::{HotKey, RawMods};
use crate::keyboard::{KbKey, Modifiers};

#[derive(Default, Debug)]
pub struct Menu {
    items: Vec<MenuItem>,
}

#[derive(Debug)]
enum MenuItem {
    Entry {
        name: String,
        id: u32,
        key: Option<HotKey>,
        selected: Option<bool>,
        enabled: bool,
    },
    SubMenu(String, Menu),
    Separator,
}

impl Menu {
    pub fn new() -> Menu {
        Menu { items: Vec::new() }
    }

    pub fn new_for_popup() -> Menu {
        Menu { items: Vec::new() }
    }

    pub fn add_dropdown(&mut self, menu: Menu, text: &str, _enabled: bool) {
        // TODO: implement enabled dropdown
        self.items
            .push(MenuItem::SubMenu(strip_access_key(text), menu));
    }

    pub fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        selected: Option<bool>,
        enabled: bool,
    ) {
        self.items.push(MenuItem::Entry {
            name: strip_access_key(text),
            id,
            key: key.cloned(),
            selected,
            enabled,
        });
    }

    pub fn add_separator(&mut self) {
        self.items.push(MenuItem::Separator)
    }

    fn append_items_to_menu<M: gtk::prelude::IsA<gtk::MenuShell>>(
        self,
        menu: &mut M,
        handle: &WindowHandle,
        accel_group: &AccelGroup,
    ) {
        for item in self.items {
            match item {
                MenuItem::Entry {
                    name,
                    id,
                    key,
                    selected,
                    enabled,
                } => {
                    if let Some(state) = selected {
                        let item = CheckMenuItem::with_label(&name);
                        if state {
                            item.activate();
                        }
                        add_menu_entry(menu, handle, accel_group, &item, id, key, enabled);
                    } else {
                        let entry = GtkMenuItem::with_label(&name);
                        add_menu_entry(menu, handle, accel_group, &entry, id, key, enabled);
                    }
                }
                MenuItem::SubMenu(name, submenu) => {
                    let item = GtkMenuItem::with_label(&name);
                    item.set_submenu(Some(&submenu.into_gtk_menu(handle, accel_group)));

                    menu.append(&item);
                }
                MenuItem::Separator => menu.append(&SeparatorMenuItem::new()),
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

    pub fn into_gtk_menu(self, handle: &WindowHandle, accel_group: &AccelGroup) -> GtkMenu {
        let mut menu = GtkMenu::new();
        menu.set_accel_group(Some(accel_group));

        self.append_items_to_menu(&mut menu, handle, accel_group);

        menu
    }
}

fn add_menu_entry<
    M: gtk::prelude::IsA<gtk::MenuShell>,
    I: gtk::prelude::IsA<gtk::MenuItem> + gtk::prelude::IsA<gtk::Widget>,
>(
    menu: &mut M,
    handle: &WindowHandle,
    accel_group: &AccelGroup,
    item: &I,
    id: u32,
    key: Option<HotKey>,
    enabled: bool,
) {
    item.set_sensitive(enabled);

    if let Some(k) = key {
        register_accelerator(item, accel_group, k);
    }

    let handle = handle.clone();
    item.connect_activate(move |_| {
        if let Some(state) = handle.state.upgrade() {
            state.handler.borrow_mut().command(id);
        }
    });

    menu.append(item);
}

fn register_accelerator<M: GtkMenuItemExt + WidgetExt>(
    item: &M,
    accel_group: &AccelGroup,
    menu_key: HotKey,
) {
    let gdk_keyval = match &menu_key.key {
        KbKey::Character(text) => text.chars().next().unwrap() as u32,
        k => {
            if let Some(gdk_key) = keycodes::key_to_raw_key(k) {
                *gdk_key
            } else {
                tracing::warn!("Cannot map key {:?}", k);
                return;
            }
        }
    };

    item.add_accelerator(
        "activate",
        accel_group,
        gdk_keyval,
        modifiers_to_gdk_modifier_type(menu_key.mods),
        gtk::AccelFlags::VISIBLE,
    );
}

fn modifiers_to_gdk_modifier_type(raw_modifiers: RawMods) -> ModifierType {
    let mut result = ModifierType::empty();

    let modifiers: Modifiers = raw_modifiers.into();

    result.set(ModifierType::MOD1_MASK, modifiers.alt());
    result.set(ModifierType::CONTROL_MASK, modifiers.ctrl());
    result.set(ModifierType::SHIFT_MASK, modifiers.shift());
    result.set(ModifierType::META_MASK, modifiers.meta());

    result
}
