// Copyright 2019 The Druid Authors.
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

use crate::hotkey::HotKey;
use crate::platform::menu as platform;

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
pub struct Menu(platform::Menu);

impl Menu {
    /// Create a new empty window or application menu.
    pub fn new() -> Menu {
        Menu(platform::Menu::new())
    }

    /// Create a new empty context menu.
    ///
    /// Some platforms distinguish between these types of menus, and some
    /// do not.
    pub fn new_for_popup() -> Menu {
        Menu(platform::Menu::new_for_popup())
    }

    /// Consume this `Menu`, returning the platform menu object.
    pub(crate) fn into_inner(self) -> platform::Menu {
        self.0
    }

    /// Add the provided `Menu` as a submenu of self, with the provided title.
    pub fn add_dropdown(&mut self, menu: Menu, text: &str, enabled: bool) {
        self.0.add_dropdown(menu.0, text, enabled)
    }

    /// Add an item to this menu.
    ///
    /// The `id` should uniquely identify this item. If the user selects this
    /// item, the responsible [`WindowHandler`]'s [`command()`] method will
    /// be called with this `id`. If the `enabled` argument is false, the menu
    /// item will be grayed out; the hotkey will also be disabled.
    /// If the `selected` argument is `true`, the menu will have a checkmark
    /// or platform appropriate equivalent indicating that it is currently selected.
    /// The `key` argument is an optional [`HotKey`] that will be registered
    /// with the system.
    ///
    ///
    /// [`WindowHandler`]: trait.WindowHandler.html
    /// [`command()`]: trait.WindowHandler.html#tymethod.command
    /// [`HotKey`]: struct.HotKey.html
    pub fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        enabled: bool,
        selected: bool,
    ) {
        self.0.add_item(id, text, key, enabled, selected)
    }

    /// Add a seperator to the menu.
    pub fn add_separator(&mut self) {
        self.0.add_separator()
    }
}
