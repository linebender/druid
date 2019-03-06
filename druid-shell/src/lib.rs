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

#[cfg(target_os = "windows")]
#[macro_use]
extern crate winapi;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod keycodes;
pub mod window;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows as platform;
#[cfg(target_os = "windows")]
pub use windows::paint;

#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(target_os = "macos")]
pub use mac as platform;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod web;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use web as platform;

pub use error::Error;

pub use platform::application;
pub use platform::dialog;
pub use platform::menu;
pub use platform::util;
pub use platform::win_main; // TODO: rename to "runloop"
pub use util::init;
