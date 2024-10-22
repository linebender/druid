// Copyright 2018 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Platform abstraction for Druid toolkit.
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
//!   backend, it will avoid using the Present extension.

#![warn(rustdoc::broken_intra_doc_links)]
#![allow(clippy::new_without_default)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/druid/screenshots/images/doc_logo.png"
)]
// This is overeager right now, see https://github.com/rust-lang/rust-clippy/issues/8494
#![allow(clippy::iter_overeager_cloned)]

// Rename `gtk_rs` back to `gtk`.
// This allows us to use `gtk` as the feature name.
// The `target_os` requirement is there to exclude anything `wasm` like.
#[cfg(all(
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd"),
    feature = "gtk"
))]
extern crate gtk_rs as gtk;

#[cfg(feature = "image")]
pub use piet::image_crate as image;
pub use piet::kurbo;
pub use piet_common as piet;

// Reexport the version of `raw_window_handle` we are using.
#[cfg(feature = "raw-win-handle")]
pub use raw_window_handle;

#[macro_use]
mod util;

mod application;
mod backend;
mod clipboard;
mod common_util;
mod dialog;
mod error;
mod hotkey;
mod keyboard;
mod menu;
mod mouse;
mod region;
mod scale;
mod screen;
mod window;

pub mod platform;
pub mod text;

pub use application::{AppHandler, Application};
pub use clipboard::{Clipboard, ClipboardFormat, FormatId};
pub use common_util::Counter;
pub use dialog::{FileDialogOptions, FileInfo, FileSpec};
pub use error::Error;
pub use hotkey::{HotKey, RawMods, SysMods};
pub use keyboard::{Code, IntoKey, KbKey, KeyEvent, KeyState, Location, Modifiers};
pub use menu::Menu;
pub use mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent};
pub use region::Region;
pub use scale::{Scalable, Scale, ScaledArea};
pub use screen::{Monitor, Screen};
pub use window::{
    FileDialogToken, IdleHandle, IdleToken, TextFieldToken, TimerToken, WinHandler, WindowBuilder,
    WindowHandle, WindowLevel, WindowState,
};

pub use keyboard_types;
