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

use crate::clipboard::Clipboard;
use crate::platform::application as platform;

/// A top-level handler that is not associated with any window.
///
/// This is most important on macOS, where it is entirely normal for
/// an application to exist with no open windows.
///
/// # Note
///
/// This is currently very limited in its functionality, and is currently
/// designed to address a single case, which is handling menu commands when
/// no window is open.
///
/// It is possible that this will expand to cover additional functionality
/// in the future.
pub trait AppHandler {
    /// Called when a menu item is selected.
    #[allow(unused_variables)]
    fn command(&mut self, id: u32) {}
}

//TODO: we may want to make the user create an instance of this (Application::global()?)
//but for now I'd like to keep changes minimal.
/// The top level application object.
pub struct Application(platform::Application);

impl Application {
    pub fn new(handler: Option<Box<dyn AppHandler>>) -> Application {
        Application(platform::Application::new(handler))
    }

    /// Start the runloop.
    ///
    /// This will block the current thread until the program has finished executing.
    pub fn run(&mut self) {
        self.0.run()
    }

    /// Terminate the application.
    pub fn quit() {
        platform::Application::quit()
    }

    // TODO: do these two go in some kind of PlatformExt trait?
    /// Hide the application this window belongs to. (cmd+H)
    pub fn hide() {
        #[cfg(target_os = "macos")]
        platform::Application::hide()
    }

    /// Hide all other applications. (cmd+opt+H)
    pub fn hide_others() {
        #[cfg(target_os = "macos")]
        platform::Application::hide_others()
    }

    /// Returns a handle to the system clipboard.
    pub fn clipboard() -> Clipboard {
        platform::Application::clipboard().into()
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
