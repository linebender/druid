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

//! Pre-configured, platform appropriate menus and menu items.

use crate::{commands, LocalizedString, SysMods};

use super::*;

/// Menu items that exist on all platforms.
pub mod common {
    use super::*;
    /// 'Cut'.
    pub fn cut<T: Data>() -> MenuItem<T> {
        MenuItem::new(LocalizedString::new("common-menu-cut"))
            .command(commands::CUT)
            .hotkey(SysMods::Cmd, "x")
    }

    /// The 'Copy' menu item.
    pub fn copy<T: Data>() -> MenuItem<T> {
        MenuItem::new(LocalizedString::new("common-menu-copy"))
            .command(commands::COPY)
            .hotkey(SysMods::Cmd, "c")
    }

    /// The 'Paste' menu item.
    pub fn paste<T: Data>() -> MenuItem<T> {
        MenuItem::new(LocalizedString::new("common-menu-paste"))
            .command(commands::PASTE)
            .hotkey(SysMods::Cmd, "v")
    }

    /// The 'Undo' menu item.
    pub fn undo<T: Data>() -> MenuItem<T> {
        MenuItem::new(LocalizedString::new("common-menu-undo"))
            .command(commands::UNDO)
            .hotkey(SysMods::Cmd, "z")
    }

    /// The 'Redo' menu item.
    pub fn redo<T: Data>() -> MenuItem<T> {
        let item = MenuItem::new(LocalizedString::new("common-menu-redo")).command(commands::REDO);

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
        pub fn default<T: Data>() -> Menu<T> {
            Menu::new(LocalizedString::new("common-menu-file-menu"))
                .entry(new())
                .entry(open())
                .entry(close())
                .entry(save_ellipsis())
                .entry(save_as())
                // revert to saved?
                .entry(print())
                .entry(page_setup())
                .separator()
                .entry(exit())
        }

        /// The 'New' menu item.
        pub fn new<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-new"))
                .command(commands::NEW_FILE)
                .hotkey(SysMods::Cmd, "n")
        }

        /// The 'Open...' menu item.
        pub fn open<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-open"))
                .command(commands::SHOW_OPEN_PANEL.with(FileDialogOptions::default()))
                .hotkey(SysMods::Cmd, "o")
        }

        /// The 'Close' menu item.
        pub fn close<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-close"))
                .command(commands::CLOSE_WINDOW)
        }

        /// The 'Save' menu item.
        pub fn save<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-save"))
                .command(commands::SAVE_FILE)
                .hotkey(SysMods::Cmd, "s")
        }

        /// The 'Save...' menu item.
        ///
        /// This is used if we need to show a dialog to select save location.
        pub fn save_ellipsis<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-save-ellipsis"))
                .command(commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()))
                .hotkey(SysMods::Cmd, "s")
        }

        /// The 'Save as...' menu item.
        pub fn save_as<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-save-as"))
                .command(commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()))
                .hotkey(SysMods::CmdShift, "S")
        }

        /// The 'Print...' menu item.
        pub fn print<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-print"))
                .command(commands::PRINT)
                .hotkey(SysMods::Cmd, "p")
        }

        /// The 'Print Preview' menu item.
        pub fn print_preview<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-print-preview"))
                .command(commands::PRINT_PREVIEW)
        }

        /// The 'Page Setup...' menu item.
        pub fn page_setup<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-page-setup"))
                .command(commands::PRINT_SETUP)
        }

        /// The 'Exit' menu item.
        pub fn exit<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("win-menu-file-exit")).command(commands::QUIT_APP)
        }
    }
}

/// macOS.
pub mod mac {
    use super::*;

    /// A basic macOS menu bar.
    pub fn menu_bar<T: Data>() -> Menu<T> {
        Menu::new(LocalizedString::new(""))
            .entry(application::default())
            .entry(file::default())
    }

    /// The application menu
    pub mod application {
        use super::*;

        /// The default Application menu.
        pub fn default<T: Data>() -> Menu<T> {
            Menu::new(LocalizedString::new("macos-menu-application-menu"))
                .entry(about())
                .separator()
                .entry(preferences().enabled(false))
                .separator()
                //.entry(MenuDesc::new(LocalizedString::new("macos-menu-services")))
                .entry(hide())
                .entry(hide_others())
                .entry(show_all().enabled(false))
                .separator()
                .entry(quit())
        }

        /// The 'About App' menu item.
        pub fn about<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("macos-menu-about-app"))
                .command(commands::SHOW_ABOUT)
        }

        /// The preferences menu item.
        pub fn preferences<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("macos-menu-preferences"))
                .command(commands::SHOW_PREFERENCES)
                .hotkey(SysMods::Cmd, ",")
        }

        /// The 'Hide' builtin menu item.
        pub fn hide<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("macos-menu-hide-app"))
                .command(commands::HIDE_APPLICATION)
                .hotkey(SysMods::Cmd, "h")
        }

        /// The 'Hide Others' builtin menu item.
        pub fn hide_others<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("macos-menu-hide-others"))
                .command(commands::HIDE_OTHERS)
                .hotkey(SysMods::AltCmd, "h")
        }

        /// The 'show all' builtin menu item
        //FIXME: this doesn't work
        pub fn show_all<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("macos-menu-show-all")).command(commands::SHOW_ALL)
        }

        /// The 'Quit' menu item.
        pub fn quit<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("macos-menu-quit-app"))
                .command(commands::QUIT_APP)
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
        pub fn default<T: Data>() -> Menu<T> {
            Menu::new(LocalizedString::new("common-menu-file-menu"))
                .entry(new_file())
                .entry(open_file())
                // open recent?
                .separator()
                .entry(close())
                .entry(save().enabled(false))
                .entry(save_as().enabled(false))
                // revert to saved?
                .separator()
                .entry(page_setup().enabled(false))
                .entry(print().enabled(false))
        }

        /// The 'New Window' item.
        ///
        /// Note: depending on context, apps might show 'New', 'New Window',
        /// 'New File', or 'New...' (where the last indicates that the menu
        /// item will open a prompt). You may want to create a custom
        /// item to capture the intent of your menu, instead of using this one.
        pub fn new_file<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-new"))
                .command(commands::NEW_FILE)
                .hotkey(SysMods::Cmd, "n")
        }

        /// The 'Open...' menu item. Will display the system file-chooser.
        pub fn open_file<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-open"))
                .command(commands::SHOW_OPEN_PANEL.with(FileDialogOptions::default()))
                .hotkey(SysMods::Cmd, "o")
        }

        /// The 'Close' menu item.
        pub fn close<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-close"))
                .command(commands::CLOSE_WINDOW)
                .hotkey(SysMods::Cmd, "w")
        }

        /// The 'Save' menu item.
        pub fn save<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-save"))
                .command(commands::SAVE_FILE)
                .hotkey(SysMods::Cmd, "s")
        }

        /// The 'Save...' menu item.
        ///
        /// This is used if we need to show a dialog to select save location.
        pub fn save_ellipsis<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-save-ellipsis"))
                .command(commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()))
                .hotkey(SysMods::Cmd, "s")
        }

        /// The 'Save as...'
        pub fn save_as<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-save-as"))
                .command(commands::SHOW_SAVE_PANEL.with(FileDialogOptions::default()))
                .hotkey(SysMods::CmdShift, "S")
        }

        /// The 'Page Setup...' menu item.
        pub fn page_setup<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-page-setup"))
                .command(commands::PRINT_SETUP)
                .hotkey(SysMods::CmdShift, "P")
        }

        /// The 'Print...' menu item.
        pub fn print<T: Data>() -> MenuItem<T> {
            MenuItem::new(LocalizedString::new("common-menu-file-print"))
                .command(commands::PRINT)
                .hotkey(SysMods::Cmd, "p")
        }
    }
}
