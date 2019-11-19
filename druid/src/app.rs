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

//! Window building and app lifecycle.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::kurbo::Size;
use crate::shell::{Application, Error as PlatformError, RunLoop, WindowBuilder, WindowHandle};
use crate::win_handler::AppState;
use crate::window::{Window, WindowId};
use crate::{theme, AppDelegate, Data, DruidHandler, Env, LocalizedString, MenuDesc, Widget};

/// A function that modifies the initial environment.
type EnvSetupFn = dyn FnOnce(&mut Env);

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher<T> {
    windows: Vec<WindowDesc<T>>,
    env_setup: Option<Box<EnvSetupFn>>,
    delegate: Option<Box<dyn AppDelegate<T>>>,
}

/// A function that can create a widget.
type WidgetBuilderFn<T> = dyn Fn() -> Box<dyn Widget<T>> + 'static;

/// A description of a window to be instantiated.
///
/// This includes a function that can build the root widget, as well as other
/// window properties such as the title.
pub struct WindowDesc<T> {
    pub(crate) root_builder: Arc<WidgetBuilderFn<T>>,
    pub(crate) title: Option<LocalizedString<T>>,
    pub(crate) size: Option<Size>,
    pub(crate) menu: Option<MenuDesc<T>>,
    /// The `WindowId` that will be assigned to this window.
    ///
    /// This can be used to track a window from when it is launched and when
    /// it actually connects.
    pub id: WindowId,
}

impl<T: Data + 'static> AppLauncher<T> {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDesc<T>) -> Self {
        AppLauncher {
            windows: vec![window],
            env_setup: None,
            delegate: None,
        }
    }

    /// Provide an optional closure that will be given mutable access to
    /// the environment before launch.
    ///
    /// This can be used to set or override theme values.
    pub fn configure_env(mut self, f: impl Fn(&mut Env) + 'static) -> Self {
        self.env_setup = Some(Box::new(f));
        self
    }

    /// Set the [`AppDelegate`].
    ///
    /// [`AppDelegate`]: struct.AppDelegate.html
    pub fn delegate(mut self, delegate: impl AppDelegate<T> + 'static) -> Self {
        self.delegate = Some(Box::new(delegate));
        self
    }

    /// Initialize a minimal logger for printing logs out to stderr.
    ///
    /// Meant for use during development only.
    pub fn use_simple_logger(self) -> Self {
        simple_logger::init().ok();
        self
    }

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(mut self, data: T) -> Result<(), PlatformError> {
        Application::init();
        let mut main_loop = RunLoop::new();
        let mut env = theme::init();
        if let Some(f) = self.env_setup.take() {
            f(&mut env);
        }

        let state = AppState::new(data, env, self.delegate.take());

        for desc in self.windows {
            let window = desc.build_native(&state)?;
            window.show();
        }

        main_loop.run();
        Ok(())
    }
}

impl<T: Data + 'static> WindowDesc<T> {
    /// Create a new `WindowDesc`, taking a funciton that will generate the root
    /// [`Widget`] for this window.
    ///
    /// It is possible that a `WindowDesc` can be reused to launch multiple windows.
    ///
    /// [`Widget`]: trait.Widget.html
    pub fn new<W, F>(root: F) -> WindowDesc<T>
    where
        W: Widget<T> + 'static,
        F: Fn() -> W + 'static,
    {
        // wrap this closure in another closure that dyns the result
        // this just makes our API slightly cleaner; callers don't need to explicitly box.
        let root_builder: Arc<WidgetBuilderFn<T>> = Arc::new(move || Box::new(root()));
        WindowDesc {
            root_builder,
            title: None,
            size: None,
            menu: MenuDesc::platform_default(),
            id: WindowId::next(),
        }
    }

    /// Set the title for this window. This is a [`LocalizedString`] that will
    /// be kept up to date as the application's state changes.
    ///
    /// [`LocalizedString`]: struct.LocalizedString.html
    pub fn title(mut self, title: LocalizedString<T>) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the window size at creation
    ///
    /// You can pass in a tuple `(width, height)` or `kurbo::Size` e.g. to create a window 1000px wide and 500px high
    /// ```ignore
    /// window.window_size((1000.0, 500.0));
    /// ```
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Attempt to create a platform window from this `WindowDesc`.
    pub(crate) fn build_native(
        &self,
        state: &Rc<RefCell<AppState<T>>>,
    ) -> Result<WindowHandle, PlatformError> {
        let mut title = self
            .title
            .clone()
            .unwrap_or_else(|| LocalizedString::new("app-name"));
        title.resolve(&state.borrow().data, &state.borrow().env);
        let mut menu = self.menu.to_owned();
        let platform_menu = menu
            .as_mut()
            .map(|m| m.build_window_menu(&state.borrow().data, &state.borrow().env));

        let handler = DruidHandler::new_shared(state.clone(), self.id);

        let mut builder = WindowBuilder::new();
        builder.set_handler(Box::new(handler));
        if let Some(size) = self.size {
            builder.set_size(size);
        }
        builder.set_title(title.localized_str());
        if let Some(menu) = platform_menu {
            builder.set_menu(menu);
        }

        let root = (self.root_builder)();
        state
            .borrow_mut()
            .add_window(self.id, Window::new(root, title, menu));

        builder.build()
    }

    /// Set the menu for this window.
    pub fn menu(mut self, menu: MenuDesc<T>) -> Self {
        self.menu = Some(menu);
        self
    }
}
