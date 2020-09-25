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

//! The top-level application type.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::clipboard::Clipboard;
use crate::error::Error;
use crate::platform::application as platform;
use crate::util;

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

/// The top level application object.
///
/// This can be thought of as a reference and it can be safely cloned.
#[derive(Clone)]
pub struct Application {
    pub(crate) platform_app: platform::Application,
    state: Rc<RefCell<State>>,
}

/// Platform-independent `Application` state.
struct State {
    running: bool,
}

/// Used to ensure only one Application instance is ever created.
static APPLICATION_CREATED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// A reference object to the current `Application`, if any.
    static GLOBAL_APP: RefCell<Option<Application>> = RefCell::new(None);
}

impl Application {
    /// Create a new `Application`.
    ///
    /// # Errors
    ///
    /// Errors if an `Application` has already been created.
    ///
    /// This may change in the future. See [druid#771] for discussion.
    ///
    /// [druid#771]: https://github.com/linebender/druid/issues/771
    pub fn new() -> Result<Application, Error> {
        if APPLICATION_CREATED.compare_and_swap(false, true, Ordering::AcqRel) {
            return Err(Error::ApplicationAlreadyExists);
        }
        util::claim_main_thread();
        let platform_app = platform::Application::new()?;
        let state = Rc::new(RefCell::new(State { running: false }));
        let app = Application {
            platform_app,
            state,
        };
        GLOBAL_APP.with(|global_app| {
            *global_app.borrow_mut() = Some(app.clone());
        });
        Ok(app)
    }

    /// Get the current globally active `Application`.
    ///
    /// A globally active `Application` exists
    /// after [`new`] is called and until [`run`] returns.
    ///
    /// # Panics
    ///
    /// Panics if there is no globally active `Application`.
    /// For a non-panicking function use [`try_global`].
    ///
    /// This function will also panic if called from a non-main thread.
    ///
    /// [`new`]: #method.new
    /// [`run`]: #method.run
    /// [`try_global`]: #method.try_global
    #[inline]
    pub fn global() -> Application {
        // Main thread assertion takes place in try_global()
        Application::try_global().expect("There is no globally active Application")
    }

    /// Get the current globally active `Application`.
    ///
    /// A globally active `Application` exists
    /// after [`new`] is called and until [`run`] returns.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-main thread.
    ///
    /// [`new`]: #method.new
    /// [`run`]: #method.run
    pub fn try_global() -> Option<Application> {
        util::assert_main_thread();
        GLOBAL_APP.with(|global_app| global_app.borrow().clone())
    }

    /// Start the `Application` runloop.
    ///
    /// The provided `handler` will be used to inform of events.
    ///
    /// This will consume the `Application` and block the current thread
    /// until the `Application` has finished executing.
    ///
    /// # Panics
    ///
    /// Panics if the `Application` is already running.
    pub fn run(self, handler: Option<Box<dyn AppHandler>>) {
        // Make sure this application hasn't run() yet.
        if let Ok(mut state) = self.state.try_borrow_mut() {
            if state.running {
                panic!("Application is already running");
            }
            state.running = true;
        } else {
            panic!("Application state already borrowed");
        }

        // Run the platform application
        self.platform_app.run(handler);

        // This application is no longer active, so clear the global reference
        GLOBAL_APP.with(|global_app| {
            *global_app.borrow_mut() = None;
        });
        // .. and release the main thread
        util::release_main_thread();
    }

    /// Quit the `Application`.
    ///
    /// This will cause [`run`] to return control back to the calling function.
    ///
    /// [`run`]: #method.run
    pub fn quit(&self) {
        self.platform_app.quit()
    }

    // TODO: do these two go in some kind of PlatformExt trait?
    /// Hide the application this window belongs to. (cmd+H)
    pub fn hide(&self) {
        #[cfg(target_os = "macos")]
        self.platform_app.hide()
    }

    /// Hide all other applications. (cmd+opt+H)
    pub fn hide_others(&self) {
        #[cfg(target_os = "macos")]
        self.platform_app.hide_others()
    }

    /// Returns a handle to the system clipboard.
    pub fn clipboard(&self) -> Clipboard {
        self.platform_app.clipboard().into()
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
