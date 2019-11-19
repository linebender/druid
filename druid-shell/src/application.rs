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

//! The top-level application type.

use crate::clipboard::ClipboardItem;
use crate::platform::application as platform;

//TODO: we may want to make the user create an instance of this (Application::global()?)
//but for now I'd like to keep changes minimal.
/// The top level application object.
pub struct Application;

impl Application {
    /// Initialize the app. At the moment, this is mostly needed for hi-dpi.
    pub fn init() {
        platform::Application::init()
    }

    /// Terminate the application.
    pub fn quit() {
        platform::Application::quit()
    }

    // TODO: do these two go in some kind of PlatformExt trait?
    /// Hide the application this window belongs to. (cmd+H)
    pub fn hide() {
        #[cfg(all(target_os = "macos", not(feature = "use_gtk")))]
        platform::Application::hide()
    }

    /// Hide all other applications. (cmd+opt+H)
    pub fn hide_others() {
        #[cfg(all(target_os = "macos", not(feature = "use_gtk")))]
        platform::Application::hide_others()
    }

    /// Returns the contents of the clipboard, if any.
    pub fn get_clipboard_contents() -> Option<ClipboardItem> {
        platform::Application::get_clipboard_contents()
    }

    /// Sets the contents of the system clipboard.
    pub fn set_clipboard_contents(item: ClipboardItem) {
        platform::Application::set_clipboard_contents(item)
    }

    /// Returns the current locale string.
    ///
    /// This should a [Unicode language identifier].
    ///
    /// [Unicode language identifier]: https://unicode.org/reports/tr35/#Unicode_language_identifier
    pub fn get_locale() -> String {
        platform::Application::get_locale()
    }
}
