// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::backend::menu as backend;
use crate::hotkey::HotKey;

/// A menu object.
///
/// This may be a window menu, an application menu (macOS) or a context (right-click)
/// menu.
///
/// # Configuring menus
///
/// Currently, a menu and its items cannot be changed once created. If you need
/// to change anything about a menu (for instance, disabling or selecting items)
/// you need to create a new menu with the desired properties.
pub struct Menu(pub(crate) backend::Menu);

impl Menu {
    /// Create a new empty window or application menu.
    pub fn new() -> Menu {
        Menu(backend::Menu::new())
    }

    /// Create a new empty context menu.
    ///
    /// Some platforms distinguish between these types of menus, and some
    /// do not.
    pub fn new_for_popup() -> Menu {
        Menu(backend::Menu::new_for_popup())
    }

    /// Consume this `Menu`, returning the platform menu object.
    pub(crate) fn into_inner(self) -> backend::Menu {
        self.0
    }

    /// Add the provided `Menu` as a submenu of self, with the provided title.
    pub fn add_dropdown(&mut self, menu: Menu, text: &str, enabled: bool) {
        self.0.add_dropdown(menu.0, text, enabled)
    }

    /// Add an item to this menu.
    ///
    /// The `id` should uniquely identify this item. If the user selects this
    /// item, the responsible [`WinHandler`]'s [`command`] method will
    /// be called with this `id`. If the `enabled` argument is false, the menu
    /// item will be grayed out; the hotkey will also be disabled.
    /// If the `selected` argument is `true`, the menu will have a checkmark
    /// or platform appropriate equivalent indicating that it is currently selected.
    /// The `key` argument is an optional [`HotKey`] that will be registered
    /// with the system.
    ///
    ///
    /// [`WinHandler`]: crate::WinHandler
    /// [`command`]: crate::WinHandler::command
    pub fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        selected: Option<bool>,
        enabled: bool,
    ) {
        self.0.add_item(id, text, key, selected, enabled)
    }

    /// Add a separator to the menu.
    pub fn add_separator(&mut self) {
        self.0.add_separator()
    }
}
