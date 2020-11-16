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

//! Menus.
//!
//! # How menus work
//!
//! The types here are a generalized 'menu description'; concrete menus
//! are part of `druid-shell`.
//!
//! We deal principally with the [`MenuDesc`] type. When you create a window,
//! you can give it a `MenuDesc`, which will be turned into a concrete menu
//! object on the current platform when the window is built.
//!
//! ## Commands
//!
//! To handle an event from a menu, you assign that menu a [`Command`], and
//! handle the [`Command` event] somewhere in your widget tree. Certain
//! special events are handled by the system; these special commands are available
//! as consts in [`Selector`].
//!
//! ## Changing the menu
//!
//! To change the menu for a window, you issue a [`SET_MENU`] command, the payload
//! of which should be a new [`MenuDesc`]. The new menu will replace the old menu.
//!
//! ## The macOS app menu
//!
//! On macOS, the main menu belongs to the application, not to the window.
//!
//! In druid, whichever window is frontmost will have its menu displayed as
//! the application menu.
//!
//! # Examples
//!
//! Creating the default Application menu for macOS:
//!
//! ```
//! use druid::{Data, LocalizedString, MenuDesc, MenuItem, SysMods};
//! use druid::commands;
//!
//! fn macos_application_menu<T: Data>() -> MenuDesc<T> {
//!     MenuDesc::new(LocalizedString::new("macos-menu-application-menu"))
//!         .append(MenuItem::new(
//!             LocalizedString::new("macos-menu-about-app"),
//!             commands::SHOW_ABOUT,
//!         ))
//!         .append_separator()
//!         .append(
//!             MenuItem::new(
//!                 LocalizedString::new("macos-menu-preferences"),
//!                 commands::SHOW_PREFERENCES,
//!             )
//!             .hotkey(SysMods::Cmd, ",")
//!             .disabled(),
//!         )
//!         .append_separator()
//!         .append(MenuDesc::new(LocalizedString::new("macos-menu-services")))
//!         .append(
//!             MenuItem::new(
//!                 LocalizedString::new("macos-menu-hide-app"),
//!                 commands::HIDE_APPLICATION,
//!             )
//!             .hotkey(SysMods::Cmd, "h"),
//!         )
//!         .append(
//!             MenuItem::new(
//!                 LocalizedString::new("macos-menu-hide-others"),
//!                 commands::HIDE_OTHERS,
//!             )
//!             .hotkey(SysMods::AltCmd, "h"),
//!         )
//!         .append(
//!             MenuItem::new(
//!                 LocalizedString::new("macos-menu-show-all"),
//!                 commands::SHOW_ALL,
//!             )
//!             .disabled(),
//!         )
//!         .append_separator()
//!         .append(
//!             MenuItem::new(
//!                 LocalizedString::new("macos-menu-quit-app"),
//!                 commands::QUIT_APP,
//!             )
//!             .hotkey(SysMods::Cmd, "q"),
//!         )
//! }
//! ```
//!
//! [`MenuDesc`]: struct.MenuDesc.html
//! [`Command`]: ../struct.Command.html
//! [`Command` event]: ../enum.Event.html#variant.Command
//! [`Selector`]: ../struct.Selector.html
//! [`SET_MENU`]: ../struct.Selector.html#associatedconstant.SET_MENU

use std::num::NonZeroU32;

use crate::kurbo::Point;
use crate::shell::{HotKey, IntoKey, Menu as PlatformMenu, RawMods, SysMods};
use crate::{commands, Command, Data, Env, LocalizedString, Selector};

/// A platform-agnostic description of an application, window, or context
/// menu.
#[derive(Clone)]
pub struct MenuDesc<T> {
    item: MenuItem<T>,
    //TODO: make me an RC if we're cloning regularly?
    items: Vec<MenuEntry<T>>,
}

/// An item in a menu, which may be a normal item, a submenu, or a separator.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum MenuEntry<T> {
    Item(MenuItem<T>),
    SubMenu(MenuDesc<T>),
    Separator,
}

/// A normal menu item.
///
/// A `MenuItem` always has a title (a [`LocalizedString`]) as well a [`Command`],
/// that is sent to the application when the item is selected.
///
/// In addition, other properties can be set during construction, such as whether
/// the item is selected (checked), or enabled, or if it has a hotkey.
///
/// [`LocalizedString`]: struct.LocalizedString.html
/// [`Command`]: struct.Command.html
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

/// A menu displayed as a pop-over.
#[derive(Debug, Clone)]
pub struct ContextMenu<T> {
    pub(crate) menu: MenuDesc<T>,
    pub(crate) location: Point,
}

/// Uniquely identifies a menu item.
///
/// On the druid-shell side, the id is represented as a u32.
/// We reserve '0' as a placeholder value; on the Rust side
/// we represent this as an `Option<NonZerou32>`, which better
/// represents the semantics of our program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuItemId(Option<NonZeroU32>);

impl<T> MenuItem<T> {
    /// Create a new `MenuItem`.
    pub fn new(title: LocalizedString<T>, command: impl Into<Command>) -> Self {
        MenuItem {
            title,
            command: command.into(),
            hotkey: None,
            tool_tip: None,
            selected: false,
            enabled: true,
            platform_id: MenuItemId::PLACEHOLDER,
        }
    }

    /// A builder method that adds a hotkey for this item.
    ///
    /// # Example
    ///
    /// ```
    /// # use druid::{LocalizedString, MenuDesc, MenuItem, Selector, SysMods};
    ///
    /// let item = MenuItem::new(LocalizedString::new("My Menu Item"), Selector::new("My Selector"))
    ///     .hotkey(SysMods::Cmd, "m");
    ///
    /// # // hide the type param in or example code by letting it be inferred here
    /// # MenuDesc::<u32>::empty().append(item);
    /// ```
    pub fn hotkey(mut self, mods: impl Into<Option<RawMods>>, key: impl IntoKey) -> Self {
        self.hotkey = Some(HotKey::new(mods, key));
        self
    }

    /// Disable this menu item.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Disable this menu item if the provided predicate is true.
    pub fn disabled_if(mut self, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.enabled = false;
        }
        self
    }

    /// Mark this menu item as selected. This will usually be indicated by
    /// a checkmark.
    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    /// Mark this item as selected, if the provided predicate is true.
    pub fn selected_if(mut self, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.selected = true;
        }
        self
    }
}

impl<T: Data> MenuDesc<T> {
    /// Create a new, empty menu.
    pub fn empty() -> Self {
        Self::new(LocalizedString::new(""))
    }

    /// Create a new menu with the given title.
    pub fn new(title: LocalizedString<T>) -> Self {
        let item = MenuItem::new(title, Selector::NOOP);
        MenuDesc {
            item,
            items: Vec::new(),
        }
    }

    /// If this platform always expects windows to have a menu by default,
    /// returns a menu. Otherwise returns `None`.
    #[allow(unreachable_code)]
    pub fn platform_default() -> Option<MenuDesc<T>> {
        #[cfg(target_os = "macos")]
        return Some(MenuDesc::empty().append(sys::mac::application::default()));
        #[cfg(target_os = "windows")]
        return None;

        // we want to explicitly handle all platforms; log if a platform is missing.
        log::warn!("MenuDesc::platform_default is not implemented for this platform.");
        None
    }

    /// Given a function that produces an iterator, appends that iterator's
    /// items to this menu.
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::{Command, LocalizedString, MenuDesc, MenuItem, Selector, Target};
    ///
    /// let num_items: usize = 4;
    /// const MENU_COUNT_ACTION: Selector<usize> = Selector::new("menu-count-action");
    ///
    /// let my_menu: MenuDesc<u32> = MenuDesc::empty()
    ///     .append_iter(|| (0..num_items).map(|i| {
    ///         MenuItem::new(
    ///             LocalizedString::new("hello-counter").with_arg("count", move |_, _| i.into()),
    ///             Command::new(MENU_COUNT_ACTION, i, Target::Auto),
    ///        )
    ///     })
    /// );
    ///
    /// assert_eq!(my_menu.len(), 4);
    /// ```
    pub fn append_iter<I: Iterator<Item = MenuItem<T>>>(mut self, f: impl FnOnce() -> I) -> Self {
        for item in f() {
            self.items.push(item.into());
        }
        self
    }

    /// Append an item to this menu.
    pub fn append(mut self, item: impl Into<MenuEntry<T>>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Append an item to this menu if the predicate is matched.
    pub fn append_if(mut self, item: impl Into<MenuEntry<T>>, mut p: impl FnMut() -> bool) -> Self {
        if p() {
            self.items.push(item.into());
        }
        self
    }

    /// Append a separator.
    pub fn append_separator(mut self) -> Self {
        self.items.push(MenuEntry::Separator);
        self
    }

    /// The number of items in the menu.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the menu contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Build an application or window menu for the current platform.
    ///
    /// This takes self as &mut because it resolves localization.
    pub(crate) fn build_window_menu(&mut self, data: &T, env: &Env) -> PlatformMenu {
        self.build_native_menu(data, env, false)
    }

    /// Build a popup menu for the current platform.
    ///
    /// This takes self as &mut because it resolves localization.
    pub(crate) fn build_popup_menu(&mut self, data: &T, env: &Env) -> PlatformMenu {
        self.build_native_menu(data, env, true)
    }

    /// impl shared for window & context menus
    fn build_native_menu(&mut self, data: &T, env: &Env, for_popup: bool) -> PlatformMenu {
        let mut menu = if for_popup {
            PlatformMenu::new_for_popup()
        } else {
            PlatformMenu::new()
        };
        for item in &mut self.items {
            match item {
                MenuEntry::Item(ref mut item) => {
                    item.title.resolve(data, env);
                    item.platform_id = MenuItemId::next();
                    menu.add_item(
                        item.platform_id.as_u32(),
                        &item.title.localized_str(),
                        item.hotkey.as_ref(),
                        item.enabled,
                        item.selected,
                    );
                }
                MenuEntry::Separator => menu.add_separator(),
                MenuEntry::SubMenu(ref mut submenu) => {
                    let sub = submenu.build_native_menu(data, env, false);
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

    /// Given a command identifier from druid-shell, returns the command
    /// corresponding to that id in this menu, if one exists.
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

impl<T> ContextMenu<T> {
    /// Create a new `ContextMenu`.
    pub fn new(menu: MenuDesc<T>, location: Point) -> Self {
        ContextMenu { menu, location }
    }
}

impl MenuItemId {
    /// The value for a menu item that has not been instantiated by
    /// the platform.
    const PLACEHOLDER: MenuItemId = MenuItemId(None);

    fn next() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static MENU_ID: AtomicU32 = AtomicU32::new(1);
        let raw = NonZeroU32::new(MENU_ID.fetch_add(2, Ordering::Relaxed));
        MenuItemId(raw)
    }

    fn as_u32(self) -> u32 {
        match self.0 {
            Some(val) => val.get(),
            None => 0,
        }
    }
}

impl<T> std::fmt::Debug for MenuDesc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        fn menu_debug_impl<T>(
            menu: &MenuDesc<T>,
            f: &mut std::fmt::Formatter,
            level: usize,
        ) -> std::fmt::Result {
            static TABS: &str =
                "                                                                              ";
            let indent = &TABS[..level * 2];
            let child_indent = &TABS[..(level + 1) * 2];
            writeln!(f, "{}{}", indent, menu.item.title.key)?;
            for item in &menu.items {
                match item {
                    MenuEntry::Item(item) => writeln!(f, "{}{}", child_indent, item.title.key)?,
                    MenuEntry::Separator => writeln!(f, "{} --------- ", child_indent)?,
                    MenuEntry::SubMenu(ref menu) => menu_debug_impl(menu, f, level + 1)?,
                }
            }
            Ok(())
        }

        menu_debug_impl(self, f, 0)
    }
}

impl<T> From<MenuItem<T>> for MenuEntry<T> {
    fn from(src: MenuItem<T>) -> MenuEntry<T> {
        MenuEntry::Item(src)
    }
}

impl<T> From<MenuDesc<T>> for MenuEntry<T> {
    fn from(src: MenuDesc<T>) -> MenuEntry<T> {
        MenuEntry::SubMenu(src)
    }
}

/// Pre-configured, platform appropriate menus and menu items.
pub mod sys {
    use super::*;

    /// Menu items that exist on all platforms.
    pub mod common {
        use super::*;
        /// 'Cut'.
        pub fn cut<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-cut"), commands::CUT)
                .hotkey(SysMods::Cmd, "x")
        }

        /// The 'Copy' menu item.
        pub fn copy<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-copy"), commands::COPY)
                .hotkey(SysMods::Cmd, "c")
        }

        /// The 'Paste' menu item.
        pub fn paste<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-paste"), commands::PASTE)
                .hotkey(SysMods::Cmd, "v")
        }

        /// The 'Undo' menu item.
        pub fn undo<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-undo"), commands::UNDO)
                .hotkey(SysMods::Cmd, "z")
        }

        /// The 'Redo' menu item.
        pub fn redo<T: Data>() -> MenuItem<T> {
            let item = MenuItem::new(LocalizedString::new("common-menu-redo"), commands::REDO);

            #[cfg(target_os = "windows")]
            {
                item.hotkey(SysMods::Cmd, "y")
            }
            #[cfg(not(target_os = "windows"))]
            {
                item.hotkey(SysMods::CmdShift, "Z")
            }
        }
    }

    /// Windows.
    pub mod win {
        use super::*;

        /// The 'File' menu.
        ///
        /// These items are taken from [the win32 documentation][].
        ///
        /// [the win32 documentation]: https://docs.microsoft.com/en-us/windows/win32/uxguide/cmd-menus#standard-menus
        pub mod file {
            use super::*;
            use crate::FileDialogOptions;

            /// A default file menu.
            ///
            /// This will not be suitable for many applications; you should
            /// build the menu you need manually, using the items defined here
            /// where appropriate.
            pub fn default<T: Data>() -> MenuDesc<T> {
                MenuDesc::new(LocalizedString::new("common-menu-file-menu"))
                    .append(new())
                    .append(open())
                    .append(close())
                    .append(save_ellipsis())
                    .append(save_as())
                    // revert to saved?
                    .append(print().disabled())
                    .append(page_setup().disabled())
                    .append_separator()
                    .append(exit())
            }

            /// The 'New' menu item.
            pub fn new<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-new"),
                    commands::NEW_FILE,
                )
                .hotkey(SysMods::Cmd, "n")
            }

            /// The 'Open...' menu item.
            pub fn open<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-open"),
                    commands::SHOW_OPEN_PANEL.with(FileDialogOptions::default()),
                )
                .hotkey(SysMods::Cmd, "o")
            }

            /// The 'Close' menu item.
            pub fn close<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-close"),
                    commands::CLOSE_WINDOW,
                )
            }

            /// The 'Save' menu item.
            pub fn save<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-save"),
                    commands::SAVE_FILE.with(None),
                )
                .hotkey(SysMods::Cmd, "s")
            }

            /// The 'Save...' menu item.
            ///
            /// This is used if we need to show a dialog to select save location.
            pub fn save_ellipsis<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-save-ellipsis"),
                    commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()),
                )
                .hotkey(SysMods::Cmd, "s")
            }

            /// The 'Save as...' menu item.
            pub fn save_as<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-save-as"),
                    commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()),
                )
                .hotkey(SysMods::CmdShift, "S")
            }

            /// The 'Print...' menu item.
            pub fn print<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-print"),
                    commands::PRINT,
                )
                .hotkey(SysMods::Cmd, "p")
            }

            /// The 'Print Preview' menu item.
            pub fn print_preview<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-print-preview"),
                    commands::PRINT_PREVIEW,
                )
            }

            /// The 'Page Setup...' menu item.
            pub fn page_setup<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-page-setup"),
                    commands::PRINT_SETUP,
                )
            }

            /// The 'Exit' menu item.
            pub fn exit<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("win-menu-file-exit"),
                    commands::QUIT_APP,
                )
            }
        }
    }

    /// macOS.
    pub mod mac {
        use super::*;

        /// A basic macOS menu bar.
        pub fn menu_bar<T: Data>() -> MenuDesc<T> {
            MenuDesc::new(LocalizedString::new(""))
                .append(application::default())
                .append(file::default())
        }

        /// The application menu
        pub mod application {
            use super::*;

            /// The default Application menu.
            pub fn default<T: Data>() -> MenuDesc<T> {
                MenuDesc::new(LocalizedString::new("macos-menu-application-menu"))
                    .append(about())
                    .append_separator()
                    .append(preferences().disabled())
                    .append_separator()
                    //.append(MenuDesc::new(LocalizedString::new("macos-menu-services")))
                    .append(hide())
                    .append(hide_others())
                    .append(show_all().disabled())
                    .append_separator()
                    .append(quit())
            }

            /// The 'About App' menu item.
            pub fn about<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("macos-menu-about-app"),
                    commands::SHOW_ABOUT,
                )
            }

            /// The preferences menu item.
            pub fn preferences<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("macos-menu-preferences"),
                    commands::SHOW_PREFERENCES,
                )
                .hotkey(SysMods::Cmd, ",")
            }

            /// The 'Hide' builtin menu item.
            pub fn hide<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("macos-menu-hide-app"),
                    commands::HIDE_APPLICATION,
                )
                .hotkey(SysMods::Cmd, "h")
            }

            /// The 'Hide Others' builtin menu item.
            pub fn hide_others<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("macos-menu-hide-others"),
                    commands::HIDE_OTHERS,
                )
                .hotkey(SysMods::AltCmd, "h")
            }

            /// The 'show all' builtin menu item
            //FIXME: this doesn't work
            pub fn show_all<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("macos-menu-show-all"),
                    commands::SHOW_ALL,
                )
            }

            /// The 'Quit' menu item.
            pub fn quit<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("macos-menu-quit-app"),
                    commands::QUIT_APP,
                )
                .hotkey(SysMods::Cmd, "q")
            }
        }
        /// The file menu.
        pub mod file {
            use super::*;
            use crate::FileDialogOptions;

            /// A default file menu.
            ///
            /// This will not be suitable for many applications; you should
            /// build the menu you need manually, using the items defined here
            /// where appropriate.
            pub fn default<T: Data>() -> MenuDesc<T> {
                MenuDesc::new(LocalizedString::new("common-menu-file-menu"))
                    .append(new_file())
                    .append(open_file())
                    // open recent?
                    .append_separator()
                    .append(close())
                    .append(save().disabled())
                    .append(save_as().disabled())
                    // revert to saved?
                    .append_separator()
                    .append(page_setup().disabled())
                    .append(print().disabled())
            }

            /// The 'New Window' item.
            ///
            /// Note: depending on context, apps might show 'New', 'New Window',
            /// 'New File', or 'New...' (where the last indicates that the menu
            /// item will open a prompt). You may want to create a custom
            /// item to capture the intent of your menu, instead of using this one.
            pub fn new_file<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-new"),
                    commands::NEW_FILE,
                )
                .hotkey(SysMods::Cmd, "n")
            }

            /// The 'Open...' menu item. Will display the system file-chooser.
            pub fn open_file<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-open"),
                    commands::SHOW_OPEN_PANEL.with(FileDialogOptions::default()),
                )
                .hotkey(SysMods::Cmd, "o")
            }

            /// The 'Close' menu item.
            pub fn close<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-close"),
                    commands::CLOSE_WINDOW,
                )
                .hotkey(SysMods::Cmd, "w")
            }

            /// The 'Save' menu item.
            pub fn save<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-save"),
                    commands::SAVE_FILE.with(None),
                )
                .hotkey(SysMods::Cmd, "s")
            }

            /// The 'Save...' menu item.
            ///
            /// This is used if we need to show a dialog to select save location.
            pub fn save_ellipsis<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-save-ellipsis"),
                    commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()),
                )
                .hotkey(SysMods::Cmd, "s")
            }

            /// The 'Save as...'
            pub fn save_as<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-save-as"),
                    commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()),
                )
                .hotkey(SysMods::CmdShift, "S")
            }

            /// The 'Page Setup...' menu item.
            pub fn page_setup<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-page-setup"),
                    commands::PRINT_SETUP,
                )
                .hotkey(SysMods::CmdShift, "P")
            }

            /// The 'Print...' menu item.
            pub fn print<T: Data>() -> MenuItem<T> {
                MenuItem::new(
                    LocalizedString::new("common-menu-file-print"),
                    commands::PRINT,
                )
                .hotkey(SysMods::Cmd, "p")
            }
        }
    }
}
