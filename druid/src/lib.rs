// Copyright 2018 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Simple data-oriented GUI.
//!
//! Druid lets you build simple interactive graphical applications that
//! can be deployed on Windows, macOS, Linux, OpenBSD, FreeBSD and the web.
//!
//! Druid is built on top of [`druid-shell`], which implements all of the
//! lower-level, platform-specific code, providing a common abstraction
//! for things like key and mouse events, creating windows, and launching
//! an application. Below [`druid-shell`] is [`piet`], which is a cross-platform
//! 2D graphics library, providing a simple and familiar drawing API that can be
//! implemented for various platforms.
//!
//! Druid is a data-driven, declarative framework. You describe your application
//! model in terms of the [`Data`] trait, and then you build up a tree of
//! [`widget`] s that can display and modify your data.
//!
//! Your widgets handle [`Event`]s, such as mouse movement, and can modify the data;
//! these changes are then delivered to relevant widgets, which can update
//! their state and redraw.
//!
//! As your application grows, you can use [`Lens`]es to expose only certain
//! subsets of your data model to certain subsets of your widget tree.
//!
//! For more information you should read the [Druid book].
//!
//! # Examples
//!
//! For many more examples, see [`druid/examples`].
//!
//! ```no_run
//! use druid::widget::{Align, Flex, Label, TextBox};
//! use druid::{AppLauncher, Data, Env, Lens, LocalizedString, Widget, WindowDesc, WidgetExt};
//!
//! const VERTICAL_WIDGET_SPACING: f64 = 20.0;
//! const TEXT_BOX_WIDTH: f64 = 200.0;
//! const WINDOW_TITLE: LocalizedString<HelloState> = LocalizedString::new("Hello World!");
//!
//! #[derive(Clone, Data, Lens)]
//! struct HelloState {
//!     name: String,
//! }
//!
//! fn main() {
//!     // describe the main window
//!     let main_window = WindowDesc::new(build_root_widget())
//!         .title(WINDOW_TITLE)
//!         .window_size((400.0, 400.0));
//!
//!     // create the initial app state
//!     let initial_state = HelloState {
//!         name: "World".into(),
//!     };
//!
//!     // start the application
//!     AppLauncher::with_window(main_window)
//!         .launch(initial_state)
//!         .expect("Failed to launch application");
//! }
//!
//! fn build_root_widget() -> impl Widget<HelloState> {
//!     // a label that will determine its text based on the current app data.
//!     let label = Label::new(|data: &HelloState, _env: &Env| format!("Hello {}!", data.name));
//!     // a textbox that modifies `name`.
//!     let textbox = TextBox::new()
//!         .with_placeholder("Who are we greeting?")
//!         .fix_width(TEXT_BOX_WIDTH)
//!         .lens(HelloState::name);
//!
//!     // arrange the two widgets vertically, with some padding
//!     let layout = Flex::column()
//!         .with_child(label)
//!         .with_spacer(VERTICAL_WIDGET_SPACING)
//!         .with_child(textbox);
//!
//!     // center the two widgets in the available space
//!     Align::centered(layout)
//! }
//! ```
//!
//! # Optional Features
//!
//! Utility features:
//!
//! * `im` - Efficient immutable data structures using the [`im` crate],
//!          which is made available via the [`im` module].
//! * `svg` - Scalable Vector Graphics for icons and other scalable images using the [`usvg` crate].
//! * `image` - Bitmap image support using the [`image` crate].
//! * `x11` - Work-in-progress X11 backend instead of GTK.
//! * `wayland` - Work-in-progress Wayland backend, very experimental.
//! * `serde` - Serde support for some internal types (most Kurbo primitives).
//!
//! Image format features:
//!
//! - png
//! - jpeg
//! - gif
//! - bmp
//! - ico
//! - tiff
//! - webp
//! - pnm
//! - dds
//! - tga
//! - hdr
//!
//! You can enable all these formats with `image-all`.
//!
//! Features can be added with `cargo`. For example, in your `Cargo.toml`:
//! ```no_compile
//! [dependencies.druid]
//! version = "0.8.3"
//! features = ["im", "svg", "image"]
//! ```
//!
//! # Note for Windows apps
//!
//! By default, Windows will open a console with your application's window. If you don't want
//! the console to be shown, use `#![windows_subsystem = "windows"]` at the beginning of your
//! crate.
//!
//! [`druid-shell`]: druid_shell
//! [`druid/examples`]: https://github.com/linebender/druid/tree/v0.8.3/druid/examples
//! [Druid book]: https://linebender.org/druid/
//! [`im` crate]: https://crates.io/crates/im
//! [`im` module]: im/index.html
//! [`usvg` crate]: https://crates.io/crates/usvg
//! [`image` crate]: https://crates.io/crates/image

#![deny(
    rustdoc::broken_intra_doc_links,
    unsafe_code,
    clippy::trivially_copy_pass_by_ref
)]
#![warn(missing_docs)]
#![allow(clippy::new_ret_no_self, clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/linebender/druid/screenshots/images/doc_logo.png"
)]

// Allows to use macros from druid_derive in this crate
extern crate self as druid;
pub use druid_derive::Lens;

use druid_shell as shell;
#[doc(inline)]
pub use druid_shell::{kurbo, piet};

// the im crate provides immutable data structures that play well with druid
#[cfg(feature = "im")]
#[doc(inline)]
pub use im;

#[macro_use]
pub mod lens;

#[macro_use]
mod util;

mod app;
mod app_delegate;
mod bloom;
mod box_constraints;
mod command;
mod contexts;
mod core;
mod data;
pub mod debug_state;
mod dialog;
pub mod env;
mod event;
mod ext_event;
mod localization;
pub mod menu;
mod mouse;
pub mod scroll_component;
mod sub_window;
#[cfg(not(target_arch = "wasm32"))]
pub mod tests;
pub mod text;
pub mod theme;
pub mod widget;
mod win_handler;
mod window;

// Types from kurbo & piet that are required by public API.
pub use kurbo::{Affine, Insets, Point, Rect, RoundedRectRadii, Size, Vec2};
pub use piet::{Color, ImageBuf, LinearGradient, RadialGradient, RenderContext, UnitPoint};

// these are the types from shell that we expose; others we only use internally.
#[cfg(feature = "image")]
pub use shell::image;
pub use shell::keyboard_types;
pub use shell::{
    Application, Clipboard, ClipboardFormat, Code, Cursor, CursorDesc, Error as PlatformError,
    FileInfo, FileSpec, FormatId, HotKey, KbKey, KeyEvent, Location, Modifiers, Monitor,
    MouseButton, MouseButtons, RawMods, Region, Scalable, Scale, ScaledArea, Screen, SysMods,
    TimerToken, WindowHandle, WindowLevel, WindowState,
};

#[cfg(feature = "raw-win-handle")]
pub use crate::shell::raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

pub use crate::core::{WidgetPod, WidgetState};
pub use app::{AppLauncher, WindowConfig, WindowDesc, WindowSizePolicy};
pub use app_delegate::{AppDelegate, DelegateCtx};
pub use box_constraints::BoxConstraints;
pub use command::{sys as commands, Command, Notification, Selector, SingleUse, Target};
pub use contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, UpdateCtx};
pub use data::*; // Wildcard because rustdoc has trouble inlining docs of two things called Data
pub use dialog::FileDialogOptions;
#[doc(inline)]
pub use env::{Env, Key, KeyOrValue, Value, ValueType, ValueTypeError};
pub use event::{Event, InternalEvent, InternalLifeCycle, LifeCycle, ViewContext};
pub use ext_event::{ExtEventError, ExtEventSink};
pub use lens::{Lens, LensExt};
pub use localization::LocalizedString;
#[doc(inline)]
pub use menu::{sys as platform_menus, Menu, MenuItem};
pub use mouse::MouseEvent;
pub use util::Handled;
pub use widget::{Widget, WidgetExt, WidgetId};
pub use win_handler::DruidHandler;
pub use window::{Window, WindowId};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use event::{DebugStateCell, StateCell, StateCheckFn};

#[doc(hidden)]
#[deprecated(since = "0.8.0", note = "import from druid::text module instead")]
pub use piet::{FontFamily, FontStyle, FontWeight, TextAlignment};
#[doc(hidden)]
#[deprecated(since = "0.8.0", note = "import from druid::text module instead")]
pub use text::{ArcStr, FontDescriptor, TextLayout};

/// The meaning (mapped value) of a keypress.
///
/// Note that in previous versions, the `KeyCode` field referred to the
/// physical position of the key, rather than the mapped value. In most
/// cases, applications should dispatch based on the value instead. This
/// alias is provided to make that transition easy, but in any case make
/// an explicit choice whether to use meaning or physical location and
/// use the appropriate type.
#[doc(hidden)]
#[deprecated(since = "0.7.0", note = "Use KbKey instead")]
pub type KeyCode = KbKey;

#[doc(hidden)]
#[deprecated(since = "0.7.0", note = "Use Modifiers instead")]
/// See [`Modifiers`](struct.Modifiers.html).
pub type KeyModifiers = Modifiers;
