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

//! Window building and app lifecycle.

use crate::ext_event::{ExtEventHost, ExtEventSink};
use crate::kurbo::{Point, Size};
use crate::shell::{Application, Error as PlatformError, WindowBuilder, WindowHandle, WindowLevel};
use crate::widget::LabelText;
use crate::win_handler::{AppHandler, AppState};
use crate::window::WindowId;
use crate::{AppDelegate, Data, Env, LocalizedString, MenuDesc, Widget, WidgetId};

use druid_shell::WindowState;

/// A function that modifies the initial environment.
type EnvSetupFn<T> = dyn FnOnce(&mut Env, &T);

/// Handles initial setup of an application, and starts the runloop.
pub struct AppLauncher<T> {
    windows: Vec<WindowDesc<T>>,
    env_setup: Option<Box<EnvSetupFn<T>>>,
    l10n_resources: Option<(Vec<String>, String)>,
    delegate: Option<Box<dyn AppDelegate<T>>>,
    ext_event_host: ExtEventHost,
}

/// Defines how a windows size should be determined
#[derive(Copy, Clone, Debug)]
pub enum WindowSizePolicy {
    /// Use the content of the window to determine the size.
    ///
    /// If you use this option, your root widget will be passed infinite constraints;
    /// you are responsible for ensuring that your content picks an appropriate size.
    Content,
    /// Use the provided window size
    User,
}

/// Window configuration that can be applied to a WindowBuilder, or to an existing WindowHandle.
/// It does not include anything related to app data.
#[derive(Debug)]
pub struct WindowConfig {
    pub(crate) size_policy: WindowSizePolicy,
    pub(crate) size: Option<Size>,
    pub(crate) min_size: Option<Size>,
    pub(crate) position: Option<Point>,
    pub(crate) resizable: Option<bool>,
    pub(crate) show_titlebar: Option<bool>,
    pub(crate) level: Option<WindowLevel>,
    pub(crate) state: Option<WindowState>,
}

/// A description of a window to be instantiated.
pub struct WindowDesc<T> {
    pub(crate) pending: PendingWindow<T>,
    pub(crate) config: WindowConfig,
    /// The `WindowId` that will be assigned to this window.
    ///
    /// This can be used to track a window from when it is launched and when
    /// it actually connects.
    pub id: WindowId,
}

/// A description of a native window to be instatiated for a widget
/// which needs its own, instead of using the top-level one. This is
/// only needed for special cases like wgpu integration.
#[derive(Clone)]
pub struct NativeWindowDesc {
    /// Window id of the new window to create.
    pub id: WindowId,
    /// Handle to the parent window.
    pub parent: WindowHandle,
    /// Window id of the parent window. This needs to be provided manually because there is
    /// currently no internal mapping (as in, inside druid-shell) between native child windows
    /// and their window id.
    pub parent_id: WindowId,
    /// Widget id of the widget the new window is for. The widget will receive a
    /// NATIVE_WINDOW_CONNECTED event once the child window is created.
    pub widget_id: WidgetId,
    /// The size of the child window. This is mandatory, because the OS generally doesn't allow
    /// auto-placement of child windows (unlike top-level ones, where a default position can be
    /// requested to the OS and computed by its window manager).
    pub size: Size,
    /// The position of the child window inside the client area of its parent window. If `None`,
    /// the child window is created at the origin (0,0) of the parent's client area (top left corner).
    pub position: Option<Point>,
}

/// A description of a native window layout change, deferred to its internal message loop
/// to be processed by the native platform.
#[derive(Clone)]
pub struct NativeWindowLayoutDesc {
    pub id: WindowId,
    /// Optional origin to move the window to, or `None` to keep the current platform window position.
    pub origin: Option<Point>,
    /// Optional size to resize the window, or `None` to keep the current platform window size.
    pub size: Option<Size>,
}

/// Handle to a native child window created by [`LifeCycleCtx::new_native_window()`].
///
/// Native child windows do not have an associated `Window<T>` representation. They are only
/// used internally for advanced purpose (e.g. wgpu integration). This is exposed to allow
/// adding new widgets which require a native window, but is typically only used by those and
/// never by the user.
#[derive(Clone)]
pub struct NativeWindowHandle(pub WindowHandle);

impl std::fmt::Debug for NativeWindowHandle {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        Ok(())
    }
}

/// The parts of a window, pending construction, that are dependent on top level app state
/// or are not part of the druid shells windowing abstraction.
/// This includes the boxed root widget, as well as other window properties such as the title.
pub struct PendingWindow<T> {
    pub(crate) root: Box<dyn Widget<T>>,
    pub(crate) title: LabelText<T>,
    pub(crate) menu: Option<MenuDesc<T>>,
    pub(crate) size_policy: WindowSizePolicy, // This is copied over from the WindowConfig
                                              // when the native window is constructed.
}

impl<T: Data> PendingWindow<T> {
    /// Create a pending window from any widget.
    pub fn new<W>(root: W) -> PendingWindow<T>
    where
        W: Widget<T> + 'static,
    {
        // This just makes our API slightly cleaner; callers don't need to explicitly box.
        PendingWindow {
            root: Box::new(root),
            title: LocalizedString::new("app-name").into(),
            menu: MenuDesc::platform_default(),
            size_policy: WindowSizePolicy::User,
        }
    }

    /// Set the title for this window. This is a [`LabelText`]; it can be either
    /// a `String`, a [`LocalizedString`], or a closure that computes a string;
    /// it will be kept up to date as the application's state changes.
    ///
    /// [`LabelText`]: widget/enum.LocalizedString.html
    /// [`LocalizedString`]: struct.LocalizedString.html
    pub fn title(mut self, title: impl Into<LabelText<T>>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the menu for this window.
    pub fn menu(mut self, menu: MenuDesc<T>) -> Self {
        self.menu = Some(menu);
        self
    }
}

impl<T: Data> AppLauncher<T> {
    /// Create a new `AppLauncher` with the provided window.
    pub fn with_window(window: WindowDesc<T>) -> Self {
        AppLauncher {
            windows: vec![window],
            env_setup: None,
            l10n_resources: None,
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
    ///
    /// # Panics
    ///
    /// Panics if the logger fails to initialize.
    pub fn use_simple_logger(self) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        simple_logger::SimpleLogger::new()
            .init()
            .expect("Failed to initialize logger.");
        #[cfg(target_arch = "wasm32")]
        console_log::init_with_level(log::Level::Trace).expect("Failed to initialize logger.");
        self
    }

    /// Use custom localization resource
    ///
    /// `resources` is a list of file names that contain strings. `base_dir`
    /// is a path to a directory that includes per-locale subdirectories.
    ///
    /// This directory should be of the structure `base_dir/{locale}/{resource}`,
    /// where '{locale}' is a valid BCP47 language tag, and {resource} is a `.ftl`
    /// included in `resources`.
    pub fn localization_resources(mut self, resources: Vec<String>, base_dir: String) -> Self {
        self.l10n_resources = Some((resources, base_dir));
        self
    }

    /// Returns an [`ExtEventSink`] that can be moved between threads,
    /// and can be used to submit commands back to the application.
    ///
    /// [`ExtEventSink`]: struct.ExtEventSink.html
    pub fn get_external_handle(&self) -> ExtEventSink {
        self.ext_event_host.make_sink()
    }

    /// Build the windows and start the runloop.
    ///
    /// Returns an error if a window cannot be instantiated. This is usually
    /// a fatal error.
    pub fn launch(mut self, data: T) -> Result<(), PlatformError> {
        let app = Application::new()?;

        let mut env = self
            .l10n_resources
            .map(|it| Env::with_i10n(it.0, &it.1))
            .unwrap_or_default();

        if let Some(f) = self.env_setup.take() {
            f(&mut env, &data);
        }

        let mut state = AppState::new(
            app.clone(),
            data,
            env,
            self.delegate.take(),
            self.ext_event_host,
        );

        for desc in self.windows {
            let window = desc.build_native(&mut state)?;
            window.show();
        }

        let handler = AppHandler::new(state);
        app.run(Some(Box::new(handler)));
        Ok(())
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            size_policy: WindowSizePolicy::User,
            size: None,
            min_size: None,
            position: None,
            resizable: None,
            show_titlebar: None,
            level: None,
            state: None,
        }
    }
}

impl WindowConfig {
    /// Set the window size policy.
    pub fn window_size_policy(mut self, size_policy: WindowSizePolicy) -> Self {
        self.size_policy = size_policy;
        self
    }

    /// Set the window's initial drawing area size in [display points].
    ///
    /// You can pass in a tuple `(width, height)` or a [`Size`],
    /// e.g. to create a window with a drawing area 1000dp wide and 500dp high:
    ///
    /// ```ignore
    /// window.window_size((1000.0, 500.0));
    /// ```
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// [`Size`]: struct.Size.html
    /// [display points]: struct.Scale.html
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Set the window's minimum drawing area size in [display points].
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// To set the window's initial drawing area size use [`window_size`].
    ///
    /// [`window_size`]: #method.window_size
    /// [display points]: struct.Scale.html
    pub fn with_min_size(mut self, size: impl Into<Size>) -> Self {
        self.min_size = Some(size.into());
        self
    }

    /// Set whether the window should be resizable.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    /// Set whether the window should have a titlebar and decorations.
    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.show_titlebar = Some(show_titlebar);
        self
    }

    /// Sets the window position in virtual screen coordinates.
    /// [`position`] Position in pixels.
    ///
    /// [`position`]: struct.Point.html
    pub fn set_position(mut self, position: Point) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets the [`WindowLevel`] of the window
    ///
    /// [`WindowLevel`]: enum.WindowLevel.html
    pub fn set_level(mut self, level: WindowLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Sets the [`WindowState`] of the window.
    ///
    /// [`WindowState`]: enum.WindowState.html
    pub fn set_window_state(mut self, state: WindowState) -> Self {
        self.state = Some(state);
        self
    }

    /// Apply this window configuration to the passed in WindowBuilder
    pub fn apply_to_builder(&self, builder: &mut WindowBuilder) {
        if let Some(resizable) = self.resizable {
            builder.resizable(resizable);
        }

        if let Some(show_titlebar) = self.show_titlebar {
            builder.show_titlebar(show_titlebar);
        }

        if let Some(size) = self.size {
            builder.set_size(size);
        } else if let WindowSizePolicy::Content = self.size_policy {
            builder.set_size(Size::new(0., 0.));
        }

        if let Some(position) = self.position {
            builder.set_position(position);
        }

        if let Some(level) = self.level {
            builder.set_level(level)
        }

        if let Some(state) = self.state {
            builder.set_window_state(state);
        }

        if let Some(min_size) = self.min_size {
            builder.set_min_size(min_size);
        }
    }

    /// Apply this window configuration to the passed in WindowHandle
    pub fn apply_to_handle(&self, win_handle: &mut WindowHandle) {
        if let Some(resizable) = self.resizable {
            win_handle.resizable(resizable);
        }

        if let Some(show_titlebar) = self.show_titlebar {
            win_handle.show_titlebar(show_titlebar);
        }

        if let Some(size) = self.size {
            win_handle.set_size(size);
        }

        // Can't apply min size currently as window handle
        // does not support it.

        if let Some(position) = self.position {
            win_handle.set_position(position);
        }

        if let Some(level) = self.level {
            win_handle.set_level(level)
        }

        if let Some(state) = self.state {
            win_handle.set_window_state(state);
        }
    }
}

impl<T: Data> WindowDesc<T> {
    /// Create a new `WindowDesc`, taking the root [`Widget`] for this window.
    ///
    /// [`Widget`]: trait.Widget.html
    pub fn new<W>(root: W) -> WindowDesc<T>
    where
        W: Widget<T> + 'static,
    {
        WindowDesc {
            pending: PendingWindow::new(root),
            config: WindowConfig::default(),
            id: WindowId::next(),
        }
    }

    /// Set the title for this window. This is a [`LabelText`]; it can be either
    /// a `String`, a [`LocalizedString`], or a closure that computes a string;
    /// it will be kept up to date as the application's state changes.
    ///
    /// [`LabelText`]: widget/enum.LocalizedString.html
    /// [`LocalizedString`]: struct.LocalizedString.html
    pub fn title(mut self, title: impl Into<LabelText<T>>) -> Self {
        self.pending = self.pending.title(title);
        self
    }

    /// Set the menu for this window.
    pub fn menu(mut self, menu: MenuDesc<T>) -> Self {
        self.pending = self.pending.menu(menu);
        self
    }

    /// Set the window size policy
    pub fn window_size_policy(mut self, size_policy: WindowSizePolicy) -> Self {
        self.config.size_policy = size_policy;
        self
    }

    /// Set the window's initial drawing area size in [display points].
    ///
    /// You can pass in a tuple `(width, height)` or a [`Size`],
    /// e.g. to create a window with a drawing area 1000dp wide and 500dp high:
    ///
    /// ```ignore
    /// window.window_size((1000.0, 500.0));
    /// ```
    ///
    /// The actual window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// [`Size`]: struct.Size.html
    /// [display points]: struct.Scale.html
    pub fn window_size(mut self, size: impl Into<Size>) -> Self {
        self.config.size = Some(size.into());
        self
    }

    /// Set the window's minimum drawing area size in [display points].
    ///
    /// The actual minimum window size in pixels will depend on the platform DPI settings.
    ///
    /// This should be considered a request to the platform to set the minimum size of the window.
    /// The platform might increase the size a tiny bit due to DPI.
    ///
    /// To set the window's initial drawing area size use [`window_size`].
    ///
    /// [`window_size`]: #method.window_size
    /// [display points]: struct.Scale.html
    pub fn with_min_size(mut self, size: impl Into<Size>) -> Self {
        self.config = self.config.with_min_size(size);
        self
    }

    /// Builder-style method to set whether this window can be resized.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config = self.config.resizable(resizable);
        self
    }

    /// Builder-style method to set whether this window's titlebar is visible.
    pub fn show_titlebar(mut self, show_titlebar: bool) -> Self {
        self.config = self.config.show_titlebar(show_titlebar);
        self
    }

    /// Sets the initial window position in virtual screen coordinates.
    /// [`position`] Position in pixels.
    ///
    /// [`position`]: struct.Point.html
    pub fn set_position(mut self, position: impl Into<Point>) -> Self {
        self.config = self.config.set_position(position.into());
        self
    }

    /// Sets the [`WindowLevel`] of the window
    ///
    /// [`WindowLevel`]: enum.WindowLevel.html
    pub fn set_level(mut self, level: WindowLevel) -> Self {
        self.config = self.config.set_level(level);
        self
    }

    /// Set initial state for the window.
    pub fn set_window_state(mut self, state: WindowState) -> Self {
        self.config = self.config.set_window_state(state);
        self
    }

    /// Attempt to create a platform window from this `WindowDesc`.
    pub(crate) fn build_native(
        self,
        state: &mut AppState<T>,
    ) -> Result<WindowHandle, PlatformError> {
        state.build_native_window(self.id, self.pending, self.config)
    }
}

impl NativeWindowDesc {
    pub(crate) fn new(
        id: WindowId,
        parent: &WindowHandle,
        parent_id: WindowId,
        widget_id: WidgetId,
        size: Size,
        position: Option<Point>,
    ) -> Self {
        NativeWindowDesc {
            id,
            parent: parent.clone(),
            parent_id,
            widget_id,
            size,
            position,
        }
    }

    /// Attempt to create a platform window from this `NativeWindowDesc`.
    pub(crate) fn build_native<T: Data>(
        self,
        state: &mut AppState<T>,
    ) -> Result<WindowHandle, PlatformError> {
        let child_window = state.build_native_child_window(
            self.id,
            &self.parent,
            self.parent_id,
            self.size,
            self.position,
        )?;
        Ok(child_window)
    }
}
