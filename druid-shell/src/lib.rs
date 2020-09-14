// Copyright 2018 The Druid Authors.
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

//! Platform abstraction for druid toolkit.
//!
//! `druid-shell` is an abstraction around a given platform UI & application
//! framework. It provides common types, which then defer to a platform-defined
//! implementation.
//!
//! # Env
//!
//! For testing and debugging, `druid-shell` can change its behavior based on environment
//! variables. Here is a list of environment variables that `druid-shell` supports:
//!
//! - `DRUID_SHELL_DISABLE_X11_PRESENT`: if this is set and `druid-shell` is using the `x11`
//! backend, it will avoid using the Present extension.

#![deny(intra_doc_link_resolution_failure)]
#![allow(clippy::new_without_default)]
#![deny(clippy::trivially_copy_pass_by_ref)]

pub use kurbo;
pub use piet_common as piet;

#[macro_use]
mod util;

mod application;
mod clipboard;
mod common_util;
mod dialog;
mod error;
mod hotkey;
mod keyboard;
mod menu;
mod mouse;
mod platform;
mod region;
mod scale;
mod screen;
mod window;

pub use application::{AppHandler, Application};
pub use clipboard::{Clipboard, ClipboardFormat, FormatId};
pub use common_util::Counter;
pub use dialog::{FileDialogOptions, FileInfo, FileSpec};
pub use error::Error;
pub use hotkey::{HotKey, RawMods, SysMods};
pub use keyboard::{Code, IntoKey, KbKey, KeyEvent, KeyState, Location, Modifiers};
pub use menu::Menu;
pub use mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
pub use region::Region;
pub use scale::{Scalable, Scale, ScaledArea};
pub use screen::{Monitor, Screen};
pub use window::{
    IdleHandle, IdleToken, TimerToken, WinHandler, WindowBuilder, WindowHandle, WindowLevel,
    WindowState,
};

pub use keyboard_types;
