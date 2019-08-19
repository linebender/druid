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

//! Menus, used for application/window menus as well as for context (right-click).
//! The types here are a generalized 'menu description'; concrete menus
//! are part of `druid-shell`.

use crate::{Data, Env, Command, LocalizedString, Selector};
use crate::shell::hotkey::{HotKey, KeyCompare, RawMods};

use crate::shell::menu::Menu as PlatformMenu;

#[derive(Debug, Clone)]
struct MenuItem<T> {
    title: LocalizedString<T>,
    command: Command,
    hotkey: Option<HotKey>,
    tool_tip: Option<LocalizedString<T>>,
    //highlighted: bool,
    checked: bool,
    enabled: bool, // (or state is stored elsewhere)
}

#[derive(Debug, Clone)]
enum MenuEntry<T> {
    Item(MenuItem<T>),
    SubMenu(Menu<T>),
    Separator,
}

#[derive(Debug, Clone)]
pub struct Menu<T> {
    item: MenuItem<T>,
    items: Vec<MenuEntry<T>>,
}

impl<T> MenuItem<T> {
    pub fn new(title: LocalizedString<T>, command: impl Into<Command>) -> Self {
        MenuItem {
            title: title.into(),
            command: command.into(),
            hotkey: None,
            tool_tip: None,
            checked: false,
            enabled: true,
        }
    }

    pub fn hotkey(mut self, mods: impl Into<Option<RawMods>>, key: impl Into<KeyCompare>) -> Self {
        self.hotkey = Some(HotKey::new(mods, key));
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn disabled_if(mut self, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.enabled = false;
        }
        self
    }

    pub fn checked(mut self) -> Self {
        self.checked = true;
        self
    }

    pub fn checked_if(mut self, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.checked = true;
        }
        self
    }
}

impl<T: Data> Menu<T> {
    fn new(title: LocalizedString<T>) -> Self {
        let item = MenuItem::new(title, Selector::NOOP);
        Menu {
            item,
            items: Vec::new(),
        }
    }

    fn append(mut self, item: impl Into<MenuEntry<T>>) -> Self {
        self.items.push(item.into());
        self
    }

    fn append_if(mut self, item: impl Into<MenuEntry<T>>, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.items.push(item.into());
        }
        self
    }

    fn append_separator(mut self) -> Self {
        self.items.push(MenuEntry::Separator);
        self
    }

    pub fn build(&mut self, mut id: &mut u32, env: &Env, data: &T) -> PlatformMenu {
        let mut menu = PlatformMenu::new();
        for item in &mut self.items {
            *id += 1;
            match item {
                MenuEntry::Item(ref mut item) => {
                    item.title.resolve(data, env);
                    menu.add_item(*id, item.title.localized_str(), item.hotkey.as_ref());
                }
                MenuEntry::Separator => menu.add_separator(),
                MenuEntry::SubMenu(ref mut submenu) => {
                    let sub = submenu.build(&mut id, env, data);
                    submenu.item.title.resolve(data, env);
                    menu.add_dropdown(sub, &submenu.item.title.localized_str());
                }
            }
        }
        menu
    }
}

/// Selectors sent by default menu items.
pub mod selectors {
    use crate::Selector;

    pub const SHOW_PREFERENCES: Selector = Selector::new("druid-builtin.menu-show-preferences");
    pub const SHOW_ABOUT: Selector = Selector::new("druid-builtin.menu-show-about");
    pub const HIDE_APPLICATION: Selector = Selector::new("druid-builtin.menu-hide-application");
    pub const HIDE_OTHERS: Selector = Selector::new("druid-builtin.menu-hide-others");
    pub const SHOW_ALL: Selector = Selector::new("druid-builtin.menu-show-all");
    pub const QUIT_APP: Selector = Selector::new("druid-builtin.menu-quit-app");
    pub const NEW_FILE: Selector = Selector::new("druid-builtin.menu-file-new");
    pub const OPEN_FILE: Selector = Selector::new("druid-builtin.menu-file-open");
    pub const CLOSE_WINDOW: Selector = Selector::new("druid-builtin.menu-file-close");
    pub const SAVE_FILE: Selector = Selector::new("druid-builtin.menu-file-save");
    pub const SAVE_FILE_AS: Selector = Selector::new("druid-builtin.menu-file-save-as");
    pub const PRINT_SETUP: Selector = Selector::new("druid-builtin.menu-file-print-setup");
    pub const PRINT: Selector = Selector::new("druid-builtin.menu-file-print");
}

pub fn macos_menu_bar<T: Data>() -> Menu<T> {
    Menu::new(LocalizedString::new(""))
        .append(macos_application_menu())
        .append(macos_file_menu())
}

fn macos_application_menu<T: Data>() -> Menu<T> {
    Menu::new(LocalizedString::new("macos-menu-application-menu"))
        .append(MenuItem::new(
            LocalizedString::new("macos-menu-about-app"),
            selectors::SHOW_ABOUT,
        ))
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-preferences"),
                selectors::SHOW_PREFERENCES,
            )
            .hotkey(RawMods::Meta, ","),
        )
        .append_separator()
        .append(Menu::new(LocalizedString::new("macos-menu-services")))
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-hide-app"),
                selectors::HIDE_APPLICATION,
            )
            .hotkey(RawMods::Meta, "h"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-hide-others"),
                selectors::HIDE_OTHERS,
            )
            .hotkey(RawMods::AltMeta, "h"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-show-all"),
                selectors::SHOW_ALL,
            )
            .disabled(),
        )
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-quit-app"),
                selectors::QUIT_APP,
            )
            .hotkey(RawMods::Meta, "q"),
        )
}

fn macos_file_menu<T: Data>() -> Menu<T> {
    Menu::new(LocalizedString::new("macos-menu-file-menu"))
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-new"),
                selectors::NEW_FILE,
            )
            .hotkey(RawMods::Meta, "n"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-open"),
                selectors::OPEN_FILE,
            )
            .hotkey(RawMods::Meta, "o"),
        )
        // open recent?
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-close"),
                selectors::CLOSE_WINDOW,
            )
            .hotkey(RawMods::Meta, "w"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-save"),
                selectors::SAVE_FILE,
            )
            .hotkey(RawMods::Meta, "s"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-save-as"),
                selectors::SAVE_FILE_AS,
            )
            .hotkey(RawMods::MetaShift, "s"),
        )
        // revert to saved?
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-page-setup"),
                selectors::PRINT_SETUP,
            )
            .hotkey(RawMods::MetaShift, "p"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-print"),
                selectors::PRINT,
            )
            .hotkey(RawMods::Meta, "p"),
        )
}

impl<T> From<MenuItem<T>> for MenuEntry<T> {
    fn from(src: MenuItem<T>) -> MenuEntry<T> {
        MenuEntry::Item(src)
    }
}

impl<T> From<Menu<T>> for MenuEntry<T> {
    fn from(src: Menu<T>) -> MenuEntry<T> {
        MenuEntry::SubMenu(src)
    }
}
