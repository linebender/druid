// Copyright 2018 The xi-editor Authors.
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

#![deny(intra_doc_link_resolution_failure)]
#![allow(clippy::new_without_default)]

pub use kurbo;
pub use piet_common as piet;

mod application;
mod clipboard;
mod common_util;
mod dialog;
mod error;
mod hotkey;
mod keyboard;
mod keycodes;
mod menu;
mod mouse;
mod platform;
mod util;
mod window;

pub use application::{AppHandler, Application};
pub use clipboard::{Clipboard, ClipboardFormat, FormatId};
pub use common_util::Counter;
pub use dialog::{FileDialogOptions, FileInfo, FileSpec};
pub use error::Error;
pub use hotkey::{HotKey, KeyCompare, RawMods, SysMods};
pub use keyboard::{KeyEvent, KeyModifiers};
pub use keycodes::KeyCode;
pub use menu::Menu;
pub use mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
pub use window::{
    IdleHandle, IdleToken, Text, TimerToken, WinHandler, WindowBuilder, WindowHandle,
};
