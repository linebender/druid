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

use crate::ext_event::{ExtEventHost, ExtEventSink};
use crate::kurbo::Size;
use crate::shell::{Application, Error as PlatformError, WindowBuilder, WindowHandle};
use crate::win_handler::{AppHandler, AppState};
use crate::window::WindowId;
use crate::{
    theme, AppDelegate, Data, DruidHandler, Env, LocalizedString, MenuDesc, Widget, WidgetExt,
};

/// A function that modifies the initial environment.
type EnvSetupFn<T> = dyn FnOnce(&mut Env, &T);

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher<T> {
    windows: Vec<WindowDesc<T>>,
    env_setup: Option<Box<EnvSetupFn<T>>>,
    delegate: Option<Box<dyn AppDelegate<T>>>,
    ext_event_host: ExtEventHost,
}

/// A description of a window to be instantiated.
///
/// This includes a function that can build the root widget, as well as other
/// window properties such as the title.
pub struct WindowDesc<T> {
    pub(crate) root: Box<dyn Widget<T>>,
    pub(crate) title: LocalizedString<T>,
    pub(crate) size: Option<Size>,
    pub(crate) min_size: Option<Size>,
    pub(crate) menu: Option<MenuDesc<T>>,
    pub(crate) resizable: bool,
    pub(crate) show_titlebar: bool,
    /// The `WindowId` that will be assigned to this window.
    ///
    /// This can be used to track a window from when it is launched and when
    /// it actually connects.
    pub id: WindowId,
}

impl<T: Data> AppLauncher<T> {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDesc<T>) -> Self {
        AppLauncher {
            windows: vec![window],
            env_setup: None,
            delegate: None,
            ext_event_host: ExtEventHost::new(),
        }
    }

    /// Provide an optional closure that will be given mutable access to
    /// the environment and immutable access to the app state before launch.
    ///
    /// This can be used to set or override theme values.
    pub fn configure_env(mut self, f: impl Fn(&mut Env, &T) + 'static) -> Self {
        self.env_setup = Some(Box::new(f));
        self
    }

    /// Set the [`AppDelegate`].
    ///
    /// [`AppDelegate`]: trait.AppDelegate.html
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

    /// Returns an [`ExtEventSink`] that can be moved between threads,
    /// and can be used to submit events back to the application.
    ///
    /// [`ExtEventSink`]: struct.ExtEventSink.html
    pub fn get_external_handle(&self) -> ExtEventSink {
        self.ext_event_host.make_sink()
    }

    /// Paint colorful rectangles for layout debugging.
    ///
    /// The rectangles are drawn around each widget's layout rect.
    #[deprecated(since = "0.5.0", note = "Use WidgetExt::debug_paint_layout instead.")]
    pub fn debug_paint_layout(self) -> Self {
        self
    }

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(mut self, data: T) -> Result<(), PlatformError> {
        let mut env = theme::init();
        if let Some(f) = self.env_setup.take() {
            f(&mut env, &data);
        }

        let mut state = AppState::new(data, env, self.delegate.take(), self.ext_event_host);
        let handler = AppHandler::new(state.clone());

        let mut app = Application::new(Some(Box::new(handler)));
        for desc in self.windows {
            let window = desc.build_native(&mut state)?;
            window.show();
        }

        app.run();
        Ok(())
    }
}

impl<T: Data> WindowDesc<T> {
    /// Create a new `WindowDesc`, taking a function that will generate the root
    /// [`Widget`] for this window.
    ///
    /// It is possible that a `WindowDesc` can be reused to launch multiple windows.
    ///
    /// [`Widget`]: trait.Widget.html
    pub fn new<W, F>(root: F) -> WindowDesc<T>
    where
        W: Widget<T> + 'static,
        F: FnOnce() -> W + 'static,
    {
        // wrap this closure in another closure that boxes the created widget.
        // this just makes our API slightly cleaner; callers don't need to explicitly box.
        WindowDesc {
            root: root().boxed(),
            title: LocalizedString::new("app-name"),
            size: None,
            min_size: None,
            menu: MenuDesc::platform_default(),
            resizable: true,
            show_titlebar: true,
            id: WindowId::next(),
        }
    }

    /// Set the title for this window. This is a [`LocalizedString`] that will
    /// be kept up to date as the application's state changes.
    ///
    /// [`LocalizedString`]: struct.LocalizedString.html
    pub fn title(mut self, title: LocalizedString<T>) -> Self {
        self.title = title;
        self
    }

    /// Set the menu for this window.
    pub fn menu(mut self, menu: MenuDesc<T>) -> Self {
        self.menu = Some(menu);
        self
    }

    /// Set the initial window size.
    ///
    /// You can pass in a tuple `(width, height)` or `kurbo::Size` e.g.
    /// to create a window 1000px wide and 500px high
    /// ```ignore
    /// window.window_size((1000.0, 500.0));
    /// ```
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Set the minimum window size.
    ///
    /// To  set the initial window size, see [`window_size`].
    ///
    /// [`window_size`]: struct.WindowDesc.html#method.window_size
    pub fn with_min_size(mut self, size: impl Into<Size>) -> Self {
        self.min_size = Some(size.into());
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.show_titlebar = show_titlebar;
        self
    }

    /// Attempt to create a platform window from this `WindowDesc`.
    pub(crate) fn build_native(
        mut self,
        state: &mut AppState<T>,
    ) -> Result<WindowHandle, PlatformError> {
        let data = state.data();
        let env = state.env();
        self.title.resolve(&data, &env);

        let platform_menu = self.menu.as_mut().map(|m| m.build_window_menu(&data, &env));

        let handler = DruidHandler::new_shared(state.clone(), self.id);

        let mut builder = WindowBuilder::new();

        builder.resizable(self.resizable);
        builder.show_titlebar(self.show_titlebar);

        builder.set_handler(Box::new(handler));
        if let Some(size) = self.size {
            builder.set_size(size);
        }
        if let Some(min_size) = self.min_size {
            builder.set_min_size(min_size);
        }

        builder.set_title(self.title.localized_str());
        if let Some(menu) = platform_menu {
            builder.set_menu(menu);
        }

        let root = self.root;
        let mut window = WindowDesc::new(|| root);
        window.title = self.title;
        window.menu = self.menu;

        state.add_window(self.id, window);

        builder.build()
    }
}
