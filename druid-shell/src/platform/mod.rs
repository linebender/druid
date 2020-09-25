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

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::*;
#[cfg(target_os = "macos")]
pub(crate) mod shared;

#[cfg(all(feature = "x11", target_os = "linux"))]
mod x11;
#[cfg(all(feature = "x11", target_os = "linux"))]
pub use x11::*;
#[cfg(all(feature = "x11", target_os = "linux"))]
pub(crate) mod shared;

#[cfg(all(not(feature = "x11"), target_os = "linux"))]
mod gtk;
#[cfg(all(not(feature = "x11"), target_os = "linux"))]
pub use self::gtk::*;
#[cfg(all(not(feature = "x11"), target_os = "linux"))]
pub(crate) mod shared;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;
