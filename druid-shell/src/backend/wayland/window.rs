// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::single_match)]

use tracing;
use wayland_protocols::xdg_shell::client::xdg_popup;
use wayland_protocols::xdg_shell::client::xdg_positioner;
use wayland_protocols::xdg_shell::client::xdg_surface;

#[cfg(feature = "raw-win-handle")]
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, WaylandWindowHandle};

use super::application::{self, Timer};
use super::{error::Error, menu::Menu, outputs, surfaces};

use crate::Region;
use crate::{
    dialog::FileDialogOptions,
    error::Error as ShellError,
    kurbo::{Insets, Point, Rect, Size},
    mouse::{Cursor, CursorDesc},
    piet::PietText,
    scale::Scale,
    text::Event,
    window::{self, FileDialogToken, TimerToken, WinHandler, WindowLevel},
    TextFieldToken,
};

pub use surfaces::idle::Handle as IdleHandle;

// holds references to the various components for a window implementation.
struct Inner {
    pub(super) id: u64,
    pub(super) decor: Box<dyn surfaces::Decor>,
    pub(super) surface: Box<dyn surfaces::Handle>,
    pub(super) outputs: Box<dyn surfaces::Outputs>,
    pub(super) popup: Box<dyn surfaces::Popup>,
    pub(super) appdata: std::sync::Weak<application::Data>,
}

#[derive(Clone)]
pub struct WindowHandle {
    inner: std::sync::Arc<Inner>,
}

impl surfaces::Outputs for WindowHandle {
    fn removed(&self, o: &outputs::Meta) {
        self.inner.outputs.removed(o)
    }

    fn inserted(&self, o: &outputs::Meta) {
        self.inner.outputs.inserted(o)
    }
}

impl surfaces::Popup for WindowHandle {
    fn surface(
        &self,
        popup: &wayland_client::Main<xdg_surface::XdgSurface>,
        pos: &wayland_client::Main<xdg_positioner::XdgPositioner>,
    ) -> Result<wayland_client::Main<xdg_popup::XdgPopup>, Error> {
        self.inner.popup.surface(popup, pos)
    }
}

impl WindowHandle {
    pub(super) fn new(
        outputs: impl Into<Box<dyn surfaces::Outputs>>,
        decor: impl Into<Box<dyn surfaces::Decor>>,
        surface: impl Into<Box<dyn surfaces::Handle>>,
        popup: impl Into<Box<dyn surfaces::Popup>>,
        appdata: impl Into<std::sync::Weak<application::Data>>,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(Inner {
                id: surfaces::GLOBAL_ID.next(),
                outputs: outputs.into(),
                decor: decor.into(),
                surface: surface.into(),
                popup: popup.into(),
                appdata: appdata.into(),
            }),
        }
    }

    pub fn id(&self) -> u64 {
        self.inner.id
    }

    pub fn show(&self) {
        tracing::debug!("show initiated");
    }

    pub fn resizable(&self, _resizable: bool) {
        tracing::warn!("resizable is unimplemented on wayland");
    }

    pub fn show_titlebar(&self, _show_titlebar: bool) {
        tracing::warn!("show_titlebar is unimplemented on wayland");
    }

    pub fn set_position(&self, _position: Point) {
        tracing::warn!("set_position is unimplemented on wayland");
    }

    pub fn set_always_on_top(&self, _always_on_top: bool) {
        // Not supported by wayland
        tracing::warn!("set_always_on_top is unimplemented on wayland");
    }

    pub fn set_mouse_pass_through(&self, _mouse_pass_thorugh: bool) {
        tracing::warn!("set_mouse_pass_through unimplemented");
    }

    pub fn set_input_region(&self, region: Option<Region>) {
        self.inner.surface.set_input_region(region);
    }

    pub fn get_position(&self) -> Point {
        tracing::warn!("get_position is unimplemented on wayland");
        Point::ZERO
    }

    pub fn content_insets(&self) -> Insets {
        Insets::from(0.)
    }

    pub fn set_size(&self, size: Size) {
        self.inner.surface.set_size(size);
    }

    pub fn get_size(&self) -> Size {
        self.inner.surface.get_size()
    }

    pub fn is_foreground_window(&self) -> bool {
        true
    }

    pub fn set_window_state(&mut self, _current_state: window::WindowState) {
        tracing::warn!("set_window_state is unimplemented on wayland");
    }

    pub fn get_window_state(&self) -> window::WindowState {
        tracing::warn!("get_window_state is unimplemented on wayland");
        window::WindowState::Maximized
    }

    pub fn handle_titlebar(&self, _val: bool) {
        tracing::warn!("handle_titlebar is unimplemented on wayland");
    }

    /// Close the window.
    pub fn close(&self) {
        if let Some(appdata) = self.inner.appdata.upgrade() {
            tracing::trace!(
                "closing window initiated {:?}",
                appdata.active_surface_id.borrow()
            );
            appdata.handles.borrow_mut().remove(&self.id());
            appdata.active_surface_id.borrow_mut().pop_front();
            self.inner.surface.release();
            tracing::trace!(
                "closing window completed {:?}",
                appdata.active_surface_id.borrow()
            );
        }
    }

    /// Hide the window.
    pub fn hide(&self) {
        tracing::warn!("hide is unimplemented on wayland");
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        tracing::warn!("bring_to_front_and_focus is unimplemented on wayland");
    }

    /// Request a new paint, but without invalidating anything.
    pub fn request_anim_frame(&self) {
        self.inner.surface.request_anim_frame();
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        self.inner.surface.invalidate();
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    pub fn invalidate_rect(&self, rect: Rect) {
        self.inner.surface.invalidate_rect(rect);
    }

    pub fn text(&self) -> PietText {
        PietText::new()
    }

    pub fn add_text_field(&self) -> TextFieldToken {
        TextFieldToken::next()
    }

    pub fn remove_text_field(&self, token: TextFieldToken) {
        self.inner.surface.remove_text_field(token);
    }

    pub fn set_focused_text_field(&self, active_field: Option<TextFieldToken>) {
        self.inner.surface.set_focused_text_field(active_field);
    }

    pub fn update_text_field(&self, _token: TextFieldToken, _update: Event) {
        // noop until we get a real text input implementation
    }

    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        let appdata = match self.inner.appdata.upgrade() {
            Some(d) => d,
            None => {
                tracing::warn!("requested timer on a window that was destroyed");
                return Timer::new(self.id(), deadline).token();
            }
        };

        let now = instant::Instant::now();
        let mut timers = appdata.timers.borrow_mut();
        let sooner = timers
            .peek()
            .map(|timer| deadline < timer.deadline())
            .unwrap_or(true);

        let timer = Timer::new(self.id(), deadline);
        timers.push(timer);

        // It is possible that the deadline has passed since it was set.
        let timeout = if deadline < now {
            std::time::Duration::ZERO
        } else {
            deadline - now
        };

        if sooner {
            appdata.timer_handle.cancel_all_timeouts();
            appdata.timer_handle.add_timeout(timeout, timer.token());
        }

        timer.token()
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(appdata) = self.inner.appdata.upgrade() {
            appdata.set_cursor(cursor);
        }
    }

    pub fn make_cursor(&self, _desc: &CursorDesc) -> Option<Cursor> {
        tracing::warn!("unimplemented make_cursor initiated");
        None
    }

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        tracing::warn!("unimplemented open_file");
        None
    }

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        tracing::warn!("unimplemented save_as");
        None
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        Some(self.inner.surface.get_idle_handle())
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        Ok(self.inner.surface.get_scale())
    }

    pub fn set_menu(&self, _menu: Menu) {
        tracing::warn!("set_menu not implement for wayland");
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        tracing::warn!("show_context_menu not implement for wayland");
    }

    pub fn set_title(&self, title: impl Into<String>) {
        self.inner.decor.set_title(title);
    }

    pub(super) fn run_idle(&self) {
        self.inner.surface.run_idle();
    }

    pub(super) fn data(&self) -> Option<std::sync::Arc<surfaces::surface::Data>> {
        self.inner.surface.data()
    }
}

impl std::cmp::PartialEq for WindowHandle {
    fn eq(&self, rhs: &Self) -> bool {
        self.id() == rhs.id()
    }
}

impl Eq for WindowHandle {}

impl std::default::Default for WindowHandle {
    fn default() -> WindowHandle {
        WindowHandle {
            inner: std::sync::Arc::new(Inner {
                id: surfaces::GLOBAL_ID.next(),
                outputs: Box::<surfaces::surface::Dead>::default(),
                decor: Box::<surfaces::surface::Dead>::default(),
                surface: Box::<surfaces::surface::Dead>::default(),
                popup: Box::<surfaces::surface::Dead>::default(),
                appdata: std::sync::Weak::new(),
            }),
        }
    }
}

#[cfg(feature = "raw-win-handle")]
unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        tracing::error!("HasRawWindowHandle trait not implemented for wasm.");
        RawWindowHandle::Wayland(WaylandWindowHandle::empty())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CustomCursor;

/// Builder abstraction for creating new windows
pub(crate) struct WindowBuilder {
    appdata: std::sync::Weak<application::Data>,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    position: Option<Point>,
    level: WindowLevel,
    state: Option<window::WindowState>,
    // pre-scaled
    size: Size,
    min_size: Option<Size>,
    resizable: bool,
    show_titlebar: bool,
}

impl WindowBuilder {
    pub fn new(app: application::Application) -> WindowBuilder {
        WindowBuilder {
            appdata: std::sync::Arc::downgrade(&app.data),
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(0.0, 0.0),
            position: None,
            level: WindowLevel::AppWindow,
            state: None,
            min_size: None,
            resizable: true,
            show_titlebar: true,
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = Some(size);
    }

    pub fn resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    pub fn show_titlebar(&mut self, show_titlebar: bool) {
        self.show_titlebar = show_titlebar;
    }

    pub fn set_always_on_top(&mut self, _always_on_top: bool) {
        // This needs to be handled manually by the user with the desktop environment.
        tracing::warn!(
            "set_always_on_top unimplemented for wayland, since wayland is more restrictive."
        );
    }

    pub fn set_transparent(&mut self, _transparent: bool) {
        tracing::warn!(
            "set_transparent unimplemented for wayland, it allows transparency by default"
        );
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = Some(position);
    }

    pub fn set_level(&mut self, level: WindowLevel) {
        self.level = level;
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        self.state = Some(state);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, ShellError> {
        if self.menu.is_some() {
            tracing::warn!("menus unimplemented for wayland");
        }

        let level = self.level.clone();

        if let WindowLevel::Modal(parent) = level {
            return self.create_popup(parent);
        }

        if let WindowLevel::DropDown(parent) = level {
            return self.create_popup(parent);
        }

        let appdata = match self.appdata.upgrade() {
            Some(d) => d,
            None => return Err(ShellError::ApplicationDropped),
        };

        let handler = self.handler.expect("must set a window handler");

        let surface =
            surfaces::toplevel::Surface::new(appdata.clone(), handler, self.size, self.min_size);

        (&surface as &dyn surfaces::Decor).set_title(self.title);

        let handle = WindowHandle::new(
            surface.clone(),
            surface.clone(),
            surface.clone(),
            surface.clone(),
            self.appdata.clone(),
        );

        if appdata
            .handles
            .borrow_mut()
            .insert(handle.id(), handle.clone())
            .is_some()
        {
            return Err(ShellError::Platform(Error::string(
                "wayland should use a unique id",
            )));
        }

        appdata
            .active_surface_id
            .borrow_mut()
            .push_front(handle.id());

        surface.with_handler({
            let handle = handle.clone();
            move |winhandle| winhandle.connect(&handle.into())
        });

        Ok(handle)
    }

    fn create_popup(self, parent: window::WindowHandle) -> Result<WindowHandle, ShellError> {
        let dim = self.min_size.unwrap_or(Size::ZERO);
        let dim = Size::new(dim.width.max(1.), dim.height.max(1.));
        let dim = Size::new(
            self.size.width.max(dim.width),
            self.size.height.max(dim.height),
        );

        let config = surfaces::popup::Config::default()
            .with_size(dim)
            .with_offset(Into::into(
                self.position.unwrap_or_else(|| Into::into((0., 0.))),
            ));

        tracing::debug!("popup {:?}", config);

        popup::create(&parent.0, &config, self.appdata, self.handler)
    }
}

#[allow(unused)]
pub mod layershell {
    use crate::error::Error as ShellError;
    use crate::window::WinHandler;

    use super::WindowHandle;
    use crate::backend::wayland::application::{Application, Data};
    use crate::backend::wayland::error::Error;
    use crate::backend::wayland::surfaces;

    /// Builder abstraction for creating new windows
    pub(crate) struct Builder {
        appdata: std::sync::Weak<Data>,
        winhandle: Option<Box<dyn WinHandler>>,
        pub(crate) config: surfaces::layershell::Config,
    }

    impl Builder {
        pub fn new(app: Application) -> Builder {
            Builder {
                appdata: std::sync::Arc::downgrade(&app.data),
                config: surfaces::layershell::Config::default(),
                winhandle: None,
            }
        }

        pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
            self.winhandle = Some(handler);
        }

        pub fn build(self) -> Result<WindowHandle, ShellError> {
            let appdata = match self.appdata.upgrade() {
                Some(d) => d,
                None => return Err(ShellError::ApplicationDropped),
            };

            let winhandle = match self.winhandle {
                Some(winhandle) => winhandle,
                None => {
                    return Err(ShellError::Platform(Error::string(
                        "window handler required",
                    )))
                }
            };

            let surface =
                surfaces::layershell::Surface::new(appdata.clone(), winhandle, self.config.clone());

            let handle = WindowHandle::new(
                surface.clone(),
                surfaces::surface::Dead,
                surface.clone(),
                surface.clone(),
                self.appdata.clone(),
            );

            if appdata
                .handles
                .borrow_mut()
                .insert(handle.id(), handle.clone())
                .is_some()
            {
                panic!("wayland should use unique object IDs");
            }
            appdata
                .active_surface_id
                .borrow_mut()
                .push_front(handle.id());

            surface.with_handler({
                let handle = handle.clone();
                move |winhandle| winhandle.connect(&handle.into())
            });

            Ok(handle)
        }
    }
}

#[allow(unused)]
pub mod popup {
    use crate::error::Error as ShellError;
    use crate::window::WinHandler;

    use super::WindowBuilder;
    use super::WindowHandle;
    use crate::backend::wayland::application::{Application, Data};
    use crate::backend::wayland::error::Error;
    use crate::backend::wayland::surfaces;

    pub(super) fn create(
        parent: &WindowHandle,
        config: &surfaces::popup::Config,
        wappdata: std::sync::Weak<Data>,
        winhandle: Option<Box<dyn WinHandler>>,
    ) -> Result<WindowHandle, ShellError> {
        let appdata = match wappdata.upgrade() {
            Some(d) => d,
            None => return Err(ShellError::ApplicationDropped),
        };

        let winhandle = match winhandle {
            Some(winhandle) => winhandle,
            None => {
                return Err(ShellError::Platform(Error::string(
                    "window handler required",
                )))
            }
        };

        // compute the initial window size.
        let updated = config.clone();
        let surface =
            match surfaces::popup::Surface::new(appdata.clone(), winhandle, updated, parent) {
                Err(cause) => return Err(ShellError::Platform(cause)),
                Ok(s) => s,
            };

        let handle = WindowHandle::new(
            surface.clone(),
            surfaces::surface::Dead,
            surface.clone(),
            surface.clone(),
            wappdata,
        );

        if appdata
            .handles
            .borrow_mut()
            .insert(handle.id(), handle.clone())
            .is_some()
        {
            panic!("wayland should use unique object IDs");
        }
        appdata
            .active_surface_id
            .borrow_mut()
            .push_front(handle.id());

        surface.with_handler({
            let handle = handle.clone();
            move |winhandle| winhandle.connect(&handle.into())
        });

        Ok(handle)
    }
}
