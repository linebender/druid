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

use std::sync::Arc;

use crate::shell::{init, runloop, Error as PlatformError, WindowBuilder};
use crate::{Data, LocalizedString, UiMain, UiState, Widget};

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher<T> {
    windows: Vec<WindowDesc<T>>,
}

/// A function that can create a widget.
///
/// This type signature is a bit hairy because this has to work with `Command`,
/// which requires things to be sync + send.
type WidgetBuilderFn<T> = dyn Fn() -> Box<dyn Widget<T>> + Send + Sync + 'static;

/// A description of a window to be instantiated.
///
/// This includes a function that can build the root widget, as well as other
/// window properties such as the title.
pub struct WindowDesc<T> {
    pub(crate) root_builder: Arc<WidgetBuilderFn<T>>,
    //TODO: more things you can configure on a window, like size and menu
    pub(crate) title: Option<LocalizedString<T>>,
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
    pub fn launch(mut self, data: T) -> Result<(), PlatformError> {
        init();

        //TODO: launch all windows, when multi-win lands
        let window = self.windows.pop().expect("launch called with no window");
        let WindowDesc {
            root_builder,
            title,
            ..
        } = window;

        //TODO: use title when multi-win lands
        let title = title.unwrap_or(LocalizedString::new("app-name-exclaim"));
        let root = root_builder();
        let state = UiState::new(root, data);

        let mut builder = WindowBuilder::new();
        builder.set_handler(Box::new(UiMain::new(state)));
        builder.set_title(title.localized_str());
        let window = builder.build()?;

        let mut main_loop = runloop::RunLoop::new();
        window.show();

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
        F: Fn() -> W + 'static + Send + Sync,
    {
        // wrap this closure in another closure that dyns the result
        // this just makes our API slightly cleaner; callers don't need to explicitly box.
        let root_builder: Arc<WidgetBuilderFn<T>> = Arc::new(move || Box::new(root()));
        WindowDesc {
            root_builder,
            title: None,
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
}
