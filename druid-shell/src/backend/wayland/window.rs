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

#![allow(clippy::single_match)]

use tracing;

use super::{
    application::{Application, ApplicationData, Timer},
    error,
    menu::Menu,
    surfaces,
};

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
    pub(super) appdata: std::sync::Weak<ApplicationData>,
}

#[derive(Clone)]
pub struct WindowHandle {
    inner: std::sync::Arc<Inner>,
}

impl WindowHandle {
    pub(super) fn new(
        decor: impl Into<Box<dyn surfaces::Decor>>,
        surface: impl Into<Box<dyn surfaces::Handle>>,
        appdata: impl Into<std::sync::Weak<ApplicationData>>,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(Inner {
                id: surfaces::GLOBAL_ID.next(),
                decor: decor.into(),
                surface: surface.into(),
                appdata: appdata.into(),
            }),
        }
    }

    pub fn id(&self) -> u64 {
        self.inner.id
    }

    pub fn show(&self) {
        tracing::info!("show initiated");
    }

    pub fn resizable(&self, resizable: bool) {
        tracing::info!("resizable initiated {:?}", resizable);
        todo!()
    }

    pub fn show_titlebar(&self, _show_titlebar: bool) {
        tracing::info!("show_titlebar initiated");
        todo!()
    }

    pub fn set_position(&self, _position: Point) {
        tracing::info!("set_position initiated");
        todo!()
    }

    pub fn get_position(&self) -> Point {
        tracing::info!("get_position initiated");
        Point::ZERO
    }

    pub fn content_insets(&self) -> Insets {
        Insets::from(0.)
    }

    pub fn set_level(&self, _level: WindowLevel) {
        log::warn!("level is unsupported on wayland");
    }

    pub fn set_size(&self, size: Size) {
        self.inner.surface.set_size(size);
    }

    pub fn get_size(&self) -> Size {
        return self.inner.surface.get_size();
    }

    pub fn set_window_state(&mut self, _current_state: window::WindowState) {
        tracing::warn!("unimplemented set_window_state initiated");
        todo!();
    }

    pub fn get_window_state(&self) -> window::WindowState {
        tracing::warn!("unimplemented get_window_state initiated");
        window::WindowState::Maximized
    }

    pub fn handle_titlebar(&self, _val: bool) {
        tracing::warn!("unimplemented handle_titlebar initiated");
        todo!();
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

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        tracing::warn!("unimplemented bring_to_front_and_focus initiated");
        todo!()
    }

    /// Request a new paint, but without invalidating anything.
    pub fn request_anim_frame(&self) {
        tracing::trace!("request_anim_frame initiated");
        self.inner.surface.request_anim_frame();
        tracing::trace!("request_anim_frame completed");
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        tracing::trace!("invalidate initiated");
        self.inner.surface.invalidate();
        tracing::trace!("invalidate completed");
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    pub fn invalidate_rect(&self, rect: Rect) {
        tracing::trace!("invalidate_rect initiated");
        self.inner.surface.invalidate_rect(rect);
        tracing::trace!("invalidate_rect completed");
    }

    pub fn text(&self) -> PietText {
        PietText::new()
    }

    pub fn add_text_field(&self) -> TextFieldToken {
        TextFieldToken::next()
    }

    pub fn remove_text_field(&self, token: TextFieldToken) {
        tracing::trace!("remove_text_field initiated");
        self.inner.surface.remove_text_field(token);
        tracing::trace!("remove_text_field completed");
    }

    pub fn set_focused_text_field(&self, active_field: Option<TextFieldToken>) {
        tracing::trace!("set_focused_text_field initiated");
        self.inner.surface.set_focused_text_field(active_field);
        tracing::trace!("set_focused_text_field completed");
    }

    pub fn update_text_field(&self, _token: TextFieldToken, _update: Event) {
        // noop until we get a real text input implementation
    }

    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        tracing::trace!("request_timer initiated");
        let appdata = match self.inner.appdata.upgrade() {
            Some(d) => d,
            None => panic!("requested timer on a window that was destroyed"),
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

        return timer.token();
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        tracing::trace!("set_cursor initiated");
        if let Some(appdata) = self.inner.appdata.upgrade() {
            appdata.set_cursor(cursor);
        }
        tracing::trace!("set_cursor completed");
    }

    pub fn make_cursor(&self, _desc: &CursorDesc) -> Option<Cursor> {
        tracing::warn!("unimplemented make_cursor initiated");
        None
    }

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        tracing::info!("open_file initiated");
        todo!()
    }

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        tracing::info!("save_as initiated");
        todo!()
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        tracing::trace!("get_idle_handle initiated");
        Some(self.inner.surface.get_idle_handle())
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        tracing::info!("get_scale initiated");
        Ok(self.inner.surface.get_scale())
    }

    pub fn set_menu(&self, _menu: Menu) {
        tracing::info!("set_menu initiated");
        todo!()
    }

    pub fn show_context_menu(&self, _menu: Menu, _pos: Point) {
        tracing::info!("show_context_menu initiated");
        todo!()
    }

    pub fn set_title(&self, title: impl Into<String>) {
        self.inner.decor.set_title(title);
    }

    pub(super) fn run_idle(&self) {
        self.inner.surface.run_idle();
    }

    #[allow(unused)]
    pub(super) fn popup<'a>(&self, s: &'a surfaces::popup::Surface) -> Result<(), error::Error> {
        self.inner.surface.popup(s)
    }

    pub(super) fn data(&self) -> Option<std::sync::Arc<surfaces::surface::Data>> {
        self.inner.surface.data()
    }
}

impl std::cmp::PartialEq for WindowHandle {
    fn eq(&self, _rhs: &Self) -> bool {
        todo!()
    }
}

impl std::default::Default for WindowHandle {
    fn default() -> WindowHandle {
        WindowHandle {
            inner: std::sync::Arc::new(Inner {
                id: surfaces::GLOBAL_ID.next(),
                decor: Box::new(surfaces::surface::Dead::default()),
                surface: Box::new(surfaces::surface::Dead::default()),
                appdata: std::sync::Weak::new(),
            }),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct CustomCursor;

/// Builder abstraction for creating new windows
pub(crate) struct WindowBuilder {
    app_data: std::sync::Weak<ApplicationData>,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    position: Option<Point>,
    level: Option<WindowLevel>,
    state: Option<window::WindowState>,
    // pre-scaled
    size: Size,
    min_size: Option<Size>,
    resizable: bool,
    show_titlebar: bool,
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app_data: std::sync::Arc::downgrade(&app.data),
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(0.0, 0.0),
            position: None,
            level: None,
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

    pub fn set_transparent(&mut self, _transparent: bool) {
        tracing::warn!(
            "set_transparent unimplemented for wayland, it allows transparency by default"
        );
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = Some(position);
    }

    pub fn set_level(&mut self, level: WindowLevel) {
        self.level = Some(level);
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
        if matches!(self.menu, Some(_)) {
            tracing::error!("menus unimplemented");
        }

        let appdata = match self.app_data.upgrade() {
            Some(d) => d,
            None => return Err(ShellError::ApplicationDropped),
        };
        let handler = self.handler.expect("must set a window handler");
        // compute the initial window size.
        let initial_size = appdata.initial_window_size(self.size);

        let surface =
            surfaces::toplevel::Surface::new(appdata.clone(), handler, initial_size, self.min_size);

        (&surface as &dyn surfaces::Decor).set_title(self.title);

        let handle = WindowHandle::new(surface.clone(), surface.clone(), self.app_data.clone());

        if let Some(_) = appdata
            .handles
            .borrow_mut()
            .insert(handle.id(), handle.clone())
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

#[allow(unused)]
pub mod layershell {
    use crate::error::Error as ShellError;
    use crate::window::WinHandler;

    use super::WindowHandle;
    use crate::backend::wayland::application::{Application, ApplicationData};
    use crate::backend::wayland::error::Error;
    use crate::backend::wayland::surfaces;

    /// Builder abstraction for creating new windows
    pub(crate) struct Builder {
        appdata: std::sync::Weak<ApplicationData>,
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

            // compute the initial window size.
            let mut updated = self.config.clone();
            updated.initial_size = appdata.initial_window_size(self.config.initial_size);

            let surface = surfaces::layershell::Surface::new(appdata.clone(), winhandle, updated);

            let handle = WindowHandle::new(
                surfaces::surface::Dead::default(),
                surface.clone(),
                self.appdata.clone(),
            );

            if let Some(_) = appdata
                .handles
                .borrow_mut()
                .insert(handle.id(), handle.clone())
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

    use super::WindowHandle;
    use crate::backend::wayland::application::{Application, ApplicationData};
    use crate::backend::wayland::error::Error;
    use crate::backend::wayland::surfaces;

    /// Builder abstraction for creating new windows
    pub(crate) struct Builder {
        appdata: std::sync::Weak<ApplicationData>,
        winhandle: Option<Box<dyn WinHandler>>,
        pub(crate) config: surfaces::popup::Config,
    }

    impl Builder {
        pub fn new(app: Application) -> Self {
            Self {
                appdata: std::sync::Arc::downgrade(&app.data),
                config: surfaces::popup::Config::default(),
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

            // compute the initial window size.
            let updated = self.config.clone();

            let surface = surfaces::popup::Surface::new(appdata.clone(), winhandle, updated, None);

            if let Err(cause) = appdata.popup(&surface) {
                return Err(ShellError::Platform(cause));
            }

            let handle = WindowHandle::new(
                surfaces::surface::Dead::default(),
                surface.clone(),
                self.appdata.clone(),
            );

            if let Some(_) = appdata
                .handles
                .borrow_mut()
                .insert(handle.id(), handle.clone())
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
