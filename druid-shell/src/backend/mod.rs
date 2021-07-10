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

//! Platform specific implementations.

// It would be clearer to use cfg_if! macros here, but that breaks rustfmt.

// #[cfg(target_os = "windows")]
// pub(crate) mod windows;

// #[cfg(target_os = "macos")]
// pub(crate) mod mac;
// #[cfg(target_os = "macos")]
// pub(crate) mod shared;

#[cfg(any(feature = "x11", feature = "gtk"))]
pub(crate) mod shared;

#[cfg(feature = "x11")]
pub(crate) mod x11;

#[cfg(feature = "gtk")]
pub(crate) mod gtk;

// #[cfg(all(not(feature = "x11"), target_os = "linux"))]
// pub(crate) mod shared;

// #[cfg(target_arch = "wasm32")]
// pub(crate) mod web;
// #[cfg(target_arch = "wasm32")]
// pub use web::*;

enum Backend {
    X11,
    GTK,
}

fn select_backend() -> Backend {
    if cfg!(unix) {
        #[cfg(feature = "x11")]
        if crate::backend::x11::application::Application::is_available() {
            tracing::debug!("Selected X11 backend");
            return Backend::X11;
        }
        #[cfg(feature = "gtk")]
        if crate::backend::gtk::application::Application::is_available() {
            tracing::debug!("Selected GTK backend");
            return Backend::GTK;
        }
        panic!("No backend for this platform enabled")
    } else if cfg!(windows) {
        #[cfg(feature = "gtk")]
        if crate::backend::gtk::application::Application::is_available() {
            tracing::debug!("Selected GTK backend");
            return Backend::GTK;
        }
        panic!("No backend for this platform enabled")
    } else {
        #[cfg(feature = "gtk")]
        if crate::backend::gtk::application::Application::is_available() {
            tracing::debug!("Selected GTK backend");
            return Backend::GTK;
        }
        panic!("No backend for this platform enabled")
    }
}

pub(crate) fn application(
) -> Result<std::rc::Rc<dyn crate::application::ApplicationBackend>, crate::Error> {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => Ok(std::rc::Rc::new(
            crate::backend::x11::application::Application::new()?,
        )),
        #[cfg(feature = "gtk")]
        Backend::GTK => Ok(std::rc::Rc::new(
            crate::backend::gtk::application::Application::new()?,
        )),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}

pub(crate) fn menu() -> Box<dyn crate::menu::MenuBackend> {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => Box::new(crate::backend::x11::menu::Menu::new()),
        #[cfg(feature = "gtk")]
        Backend::GTK => Box::new(crate::backend::gtk::menu::Menu::new()),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}

pub(crate) fn menu_for_popup() -> Box<dyn crate::menu::MenuBackend> {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => Box::new(crate::backend::x11::menu::Menu::new_for_popup()),
        #[cfg(feature = "gtk")]
        Backend::GTK => Box::new(crate::backend::gtk::menu::Menu::new_for_popup()),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}

pub(crate) fn default_winhandler() -> Box<dyn crate::window::WindowHandleBackend> {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => Box::new(crate::backend::x11::window::WindowHandle::default()),
        #[cfg(feature = "gtk")]
        Backend::GTK => Box::new(crate::backend::gtk::window::WindowHandle::default()),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}

pub(crate) fn new_windowbuilder(
    app: crate::Application,
) -> Box<dyn crate::window::WindowBuilderBackend> {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => Box::new(crate::backend::x11::window::WindowBuilder::new(
            app.backend_app,
        )),
        #[cfg(feature = "gtk")]
        Backend::GTK => Box::new(crate::backend::gtk::window::WindowBuilder::new(
            app.backend_app,
        )),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}

pub(crate) fn get_locale() -> String {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => crate::backend::x11::application::Application::get_locale(),
        #[cfg(feature = "gtk")]
        Backend::GTK => crate::backend::gtk::application::Application::get_locale(),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}
pub(crate) fn get_monitors() -> Vec<crate::Monitor> {
    match select_backend() {
        #[cfg(feature = "x11")]
        Backend::X11 => crate::backend::x11::screen::get_monitors(),
        #[cfg(feature = "gtk")]
        Backend::GTK => crate::backend::gtk::screen::get_monitors(),
        _ => panic!("UNAVAILABLE BACKEND SELECTED"),
    }
}
