// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Logic that is shared by more than one backend.

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "linux", target_os = "openbsd"))] {
        mod keyboard;
        pub use keyboard::*;
    }
}
cfg_if::cfg_if! {
    if #[cfg(all(any(target_os = "freebsd", target_os = "linux"), any(feature = "x11", feature = "wayland")))] {
        mod timer;
        pub(crate) use timer::*;
        pub(crate) mod xkb;
        pub(crate) mod linux;
    }
}
