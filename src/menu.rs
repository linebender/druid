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

use std::num::NonZeroU32;

use crate::shell::hotkey::{HotKey, KeyCompare, RawMods};
use crate::{Command, Data, Env, LocalizedString, Selector};

use crate::shell::menu::Menu as PlatformMenu;

#[derive(Debug, Clone)]
pub struct MenuItem<T> {
    title: LocalizedString<T>,
    command: Command,
    hotkey: Option<HotKey>,
    tool_tip: Option<LocalizedString<T>>,
    //highlighted: bool,
    selected: bool,
    enabled: bool, // (or state is stored elsewhere)
    /// Identifies the platform object corresponding to this item.
    platform_id: MenuItemId,
}

#[derive(Debug, Clone)]
pub enum MenuEntry<T> {
    Item(MenuItem<T>),
    SubMenu(Menu<T>),
    Separator,
}

#[derive(Clone)]
pub struct Menu<T> {
    item: MenuItem<T>,
    //TODO: make me an RC if we're cloning regularly?
    items: Vec<MenuEntry<T>>,
}

impl<T> MenuItem<T> {
    pub fn new(title: LocalizedString<T>, command: impl Into<Command>) -> Self {
        MenuItem {
            title: title.into(),
            command: command.into(),
            hotkey: None,
            tool_tip: None,
            selected: false,
            enabled: true,
            platform_id: MenuItemId::PLACEHOLDER,
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

    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    pub fn selected_if(mut self, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.selected = true;
        }
        self
    }
}

impl<T: Data> Menu<T> {
    pub fn empty() -> Self {
        Self::new(LocalizedString::new(""))
    }

    pub fn new(title: LocalizedString<T>) -> Self {
        let item = MenuItem::new(title, Selector::NOOP);
        Menu {
            item,
            items: Vec::new(),
        }
    }

    pub fn append_iter<I: Iterator<Item = MenuItem<T>>>(mut self, f: impl FnOnce() -> I) -> Self {
        for item in f() {
            self.items.push(item.into());
        }
        self
    }

    pub fn append(mut self, item: impl Into<MenuEntry<T>>) -> Self {
        self.items.push(item.into());
        self
    }

    fn append_if(mut self, item: impl Into<MenuEntry<T>>, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.items.push(item.into());
        }
        self
    }

    pub fn append_separator(mut self) -> Self {
        self.items.push(MenuEntry::Separator);
        self
    }

    pub fn build_native(&mut self, data: &T, env: &Env) -> PlatformMenu {
        //eprintln!("building {:p} {}", self, self.items.len());
        let mut menu = PlatformMenu::new();
        for item in &mut self.items {
            match item {
                MenuEntry::Item(ref mut item) => {
                    item.title.resolve(data, env);
                    item.platform_id = MenuItemId::next();
                    menu.add_item(
                        item.platform_id.as_u32(),
                        item.title.localized_str(),
                        item.hotkey.as_ref(),
                        item.enabled,
                        item.selected,
                    );
                }
                MenuEntry::Separator => menu.add_separator(),
                MenuEntry::SubMenu(ref mut submenu) => {
                    let sub = submenu.build_native(data, env);
                    submenu.item.title.resolve(data, env);
                    menu.add_dropdown(
                        sub,
                        &submenu.item.title.localized_str(),
                        submenu.item.enabled,
                    );
                }
            }
        }
        menu
    }

    pub(crate) fn command_for_id(&self, id: u32) -> Option<Command> {
        for item in &self.items {
            match item {
                MenuEntry::Item(item) if item.platform_id.as_u32() == id => {
                    return Some(item.command.clone())
                }
                MenuEntry::SubMenu(menu) => {
                    if let Some(cmd) = menu.command_for_id(id) {
                        return Some(cmd);
                    }
                }
                _ => (),
            }
        }
        None
    }
}

impl<T> std::fmt::Debug for Menu<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        menu_debug_impl(self, f, 0)
    }
}

fn menu_debug_impl<T>(
    menu: &Menu<T>,
    f: &mut std::fmt::Formatter,
    level: usize,
) -> std::fmt::Result {
    static TABS: &str =
        "                                                                              ";
    let indent = &TABS[..level * 2];
    let child_indent = &TABS[..(level + 1) * 2];
    write!(f, "{}{}\n", indent, menu.item.title.key)?;
    for item in &menu.items {
        match item {
            MenuEntry::Item(item) => write!(f, "{}{}\n", child_indent, item.title.key)?,
            MenuEntry::Separator => write!(f, "{} --------- \n", child_indent)?,
            MenuEntry::SubMenu(ref menu) => menu_debug_impl(menu, f, level + 1)?,
        }
    }
    Ok(())
}

/// Selectors sent by default menu items.
pub mod selectors {
    use crate::Selector;

    pub const SHOW_PREFERENCES: Selector = Selector::new("druid-builtin.menu-show-preferences");
    pub const SHOW_ABOUT: Selector = Selector::new("druid-builtin.menu-show-about");
    pub const HIDE_APPLICATION: Selector = Selector::HIDE_APPLICATION;
    pub const HIDE_OTHERS: Selector = Selector::HIDE_OTHERS;
    pub const SHOW_ALL: Selector = Selector::new("druid-builtin.menu-show-all");
    pub const QUIT_APP: Selector = Selector::QUIT_APP;
    pub const NEW_FILE: Selector = Selector::new("druid-builtin.menu-file-new");
    pub const OPEN_FILE: Selector = Selector::new("druid-builtin.menu-file-open");
    pub const CLOSE_WINDOW: Selector = Selector::CLOSE_WINDOW;
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
            .hotkey(RawMods::Meta, ",")
            .disabled(),
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
            .hotkey(RawMods::Meta, "o")
            .disabled(),
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
            .hotkey(RawMods::Meta, "s")
            .disabled(),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-save-as"),
                selectors::SAVE_FILE_AS,
            )
            .hotkey(RawMods::MetaShift, "s")
            .disabled(),
        )
        // revert to saved?
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-page-setup"),
                selectors::PRINT_SETUP,
            )
            .hotkey(RawMods::MetaShift, "p")
            .disabled(),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("macos-menu-file-print"),
                selectors::PRINT,
            )
            .hotkey(RawMods::Meta, "p")
            .disabled(),
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

/// Uniquely identifies a menu item.
///
/// On the druid-shell side, the id is represented as a u32.
/// We reserve '0' as a placeholder value; on the Rust side
/// we represent this as an `Option<NonZerou32>`, which better
/// represents the semantics of our program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuItemId(Option<NonZeroU32>);

impl MenuItemId {
    /// The value for a menu item that has not been instantiated by
    /// the platform.
    const PLACEHOLDER: MenuItemId = MenuItemId(None);

    fn next() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static MENU_ID: AtomicU32 = AtomicU32::new(1);
        let raw = unsafe { NonZeroU32::new_unchecked(MENU_ID.fetch_add(2, Ordering::Relaxed)) };
        MenuItemId(Some(raw))
    }

    fn as_u32(self) -> u32 {
        match self.0 {
            Some(val) => val.get(),
            None => 0,
        }
    }
}
