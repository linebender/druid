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

#![deny(intra_doc_link_resolution_failure, unsafe_code)]
#![allow(clippy::new_ret_no_self)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use druid_shell as shell;
pub use druid_shell::{kurbo, piet};

mod app;
mod app_delegate;
mod bloom;
mod box_constraints;
mod command;
mod core;
mod data;
mod env;
mod event;
pub mod lens;
mod localization;
mod menu;
mod mouse;
pub mod theme;
pub mod widget;
mod win_handler;
mod window;

// Types from kurbo & piet that are required by public API.
pub use kurbo::{Affine, Insets, Point, Rect, Size, Vec2};
pub use piet::{Color, LinearGradient, PaintBrush, RadialGradient, RenderContext, UnitPoint};
// these are the types from shell that we expose; others we only use internally.
pub use shell::{
    Application, Clipboard, ClipboardFormat, Cursor, Error as PlatformError, FileDialogOptions,
    FileInfo, FileSpec, FormatId, HotKey, KeyCode, KeyEvent, KeyModifiers, MouseButton, RawMods,
    SysMods, Text, TimerToken, WinCtx, WindowHandle,
};

pub use crate::core::{
    BoxedWidget, EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, UpdateCtx, WidgetPod,
};
pub use app::{AppLauncher, WindowDesc};
pub use app_delegate::{AppDelegate, DelegateCtx};
pub use box_constraints::BoxConstraints;
pub use command::{sys as commands, Command, Selector, Target};
pub use data::Data;
pub use env::{Env, Key, Value};
pub use event::{Event, LifeCycle, WheelEvent};
pub use lens::{Lens, LensExt, LensWrap};
pub use localization::LocalizedString;
pub use menu::{sys as platform_menus, ContextMenu, MenuDesc, MenuItem};
pub use mouse::MouseEvent;
pub use widget::{Widget, WidgetId};
pub use win_handler::DruidHandler;
pub use window::{Window, WindowId};
