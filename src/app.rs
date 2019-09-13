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

use crate::shell::window::WindowHandle;
use crate::shell::{init, runloop, Error as PlatformError, WindowBuilder};
use crate::win_handler::AppState;
use crate::window::{Window, WindowId};
use crate::{theme, Data, DruidHandler, Env, LocalizedString, Menu, Widget};

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher<T> {
    windows: Vec<WindowDesc<T>>,
}

/// A function that can create a widget.
type WidgetBuilderFn<T> = dyn Fn() -> Box<dyn Widget<T>> + 'static;

/// A function that can build a menu.
type MenuBuilderFn<T> = dyn Fn(&T, &Env) -> Menu<T> + 'static;

/// A description of a window to be instantiated.
///
/// This includes a function that can build the root widget, as well as other
/// window properties such as the title.
pub struct WindowDesc<T> {
    pub(crate) root_builder: Arc<WidgetBuilderFn<T>>,
    pub(crate) title: Option<LocalizedString<T>>,
    pub(crate) menu_builder: Option<Arc<MenuBuilderFn<T>>>,
    //TODO: more things you can configure on a window, like size?
}

impl<T: Data + 'static> AppLauncher<T> {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDesc<T>) -> Self {
        AppLauncher {
            windows: vec![window],
        }
    }

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(self, data: T) -> Result<(), PlatformError> {
        init();
        let mut main_loop = runloop::RunLoop::new();
        let env = theme::init();
        let state = AppState::new(data, env);

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
            menu_builder: None,
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

    /// Attempt to create a platform window from this `WindowDesc`.
    pub(crate) fn build_native(
        &self,
        state: &Rc<RefCell<AppState<T>>>,
    ) -> Result<WindowHandle, PlatformError> {
        let mut title = self
            .title
            .clone()
            .unwrap_or(LocalizedString::new("app-name"));
        title.resolve(&state.borrow().data, &state.borrow().env);
        let mut menu = self
            .menu_builder
            .as_ref()
            .map(|m| m(&state.borrow().data, &state.borrow().env));
        let platform_menu = menu
            .as_mut()
            .map(|m| m.build_native(&state.borrow().data, &state.borrow().env));

        let id = WindowId::new();
        let handler = DruidHandler::new_shared(state.clone(), id);

        let mut builder = WindowBuilder::new();
        builder.set_handler(Box::new(handler));
        builder.set_title(title.localized_str());
        if let Some(menu) = platform_menu {
            builder.set_menu(menu);
        }

        let root = (self.root_builder)();
        state
            .borrow_mut()
            .add_window(id, Window::new(root, title, menu));

        Ok(WindowHandle {
            inner: builder.build()?,
        })
    }

    /// Set the menu for this window.
    pub fn menu(mut self, f: impl Fn(&T, &Env) -> Menu<T> + 'static) -> Self {
        self.menu_builder = Some(Arc::new(f));
        self
    }
}
