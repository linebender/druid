// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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

#[cfg(all(
    feature = "x11",
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
mod x11;
#[cfg(all(
    feature = "x11",
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub use x11::*;
#[cfg(all(
    feature = "x11",
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub(crate) mod shared;

#[cfg(all(
    feature = "wayland",
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
mod wayland;
#[cfg(all(
    feature = "wayland",
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub use wayland::*;
#[cfg(all(
    feature = "wayland",
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub(crate) mod shared;

#[cfg(all(
    not(feature = "x11"),
    not(feature = "wayland"),
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
mod gtk;
#[cfg(all(
    not(feature = "x11"),
    not(feature = "wayland"),
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub use self::gtk::*;
#[cfg(all(
    not(feature = "x11"),
    not(feature = "wayland"),
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub(crate) mod shared;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::*;
