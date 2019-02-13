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
#[cfg(target_os = "windows")]
extern crate direct2d;
#[cfg(target_os = "windows")]
extern crate wio;

#[macro_use]
extern crate lazy_static;

pub mod error;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows as platform;

#[cfg(target_os = "windows")]
pub use windows::application;
#[cfg(target_os = "windows")]
use windows::dcomp;
#[cfg(target_os = "windows")]
pub use windows::dialog;
#[cfg(target_os = "windows")]
pub use windows::menu;
#[cfg(target_os = "windows")]
pub use windows::paint;
#[cfg(target_os = "windows")]
pub use windows::win_main;
#[cfg(target_os = "windows")]
pub use windows::window;

#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(target_os = "macos")]
pub use mac as platform;

pub use error::Error;

pub use platform::util;
pub use util::init;
