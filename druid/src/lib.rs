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

//! Simple data-oriented GUI.
//!
//! Druid lets you build simple interactive graphical applications that
//! can be deployed on windows, mac, and linux.
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
//! subsets of your data model to certains subsets of your widget tree.
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
//!     let main_window = WindowDesc::new(build_root_widget)
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
//! [`Widget`]: trait.Widget.html
//! [`Data`]: trait.Data.html
//! [`Lens`]: trait.Lens.html
//! [`widget`]: ./widget/index.html
//! [`Event`]: enum.Event.html
//! [`druid-shell`]: https://docs.rs/druid-shell
//! [`piet`]: https://docs.rs/piet
//! [`druid/examples`]: https://github.com/xi-editor/druid/tree/v0.5.0/druid/examples

#![deny(intra_doc_link_resolution_failure, unsafe_code)]
#![allow(clippy::new_ret_no_self, clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Allows to use macros from druid_derive in this crate
extern crate self as druid;
pub use druid_derive::Lens;

use druid_shell as shell;
#[doc(inline)]
pub use druid_shell::{kurbo, piet};

mod app;
mod app_delegate;
mod bloom;
mod box_constraints;
mod command;
mod contexts;
mod core;
mod data;
mod env;
mod event;
mod ext_event;
pub mod lens;
mod localization;
mod menu;
mod mouse;
#[cfg(test)]
mod tests;
mod text;
pub mod theme;
pub mod widget;
mod win_handler;
mod window;

// Types from kurbo & piet that are required by public API.
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use piet::{Color, LinearGradient, RadialGradient, RenderContext, UnitPoint};
// these are the types from shell that we expose; others we only use internally.
pub use shell::{
    Application, Clipboard, ClipboardFormat, Cursor, Error as PlatformError, FileDialogOptions,
    FileInfo, FileSpec, FormatId, HotKey, KeyCode, KeyEvent, KeyModifiers, MouseButton, RawMods,
    SysMods, Text, TimerToken, WindowHandle,
};

pub use crate::core::WidgetPod;
pub use app::{AppLauncher, WindowDesc};
pub use app_delegate::{AppDelegate, DelegateCtx};
pub use box_constraints::BoxConstraints;
pub use command::{sys as commands, Command, Selector, Target};
pub use contexts::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, Region, UpdateCtx};
pub use data::Data;
pub use env::{Env, Key, KeyOrValue, Value, ValueType};
pub use event::{Event, LifeCycle, WheelEvent};
pub use ext_event::{ExtEventError, ExtEventSink};
pub use lens::{Lens, LensExt, LensWrap};
pub use localization::LocalizedString;
pub use menu::{sys as platform_menus, ContextMenu, MenuDesc, MenuItem};
pub use mouse::MouseEvent;
pub use widget::{Widget, WidgetExt, WidgetId};
pub use win_handler::DruidHandler;
pub use window::{Window, WindowId};

#[cfg(test)]
pub(crate) use event::{StateCell, StateCheckFn};
