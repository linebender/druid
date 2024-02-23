// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! wayland platform support

// TODO: Remove this and fix the non-Send/Sync Arc issues
#![allow(clippy::arc_with_non_send_sync)]

pub mod application;
pub mod clipboard;
mod display;
pub mod error;
mod events;
pub mod keyboard;
pub mod menu;
mod outputs;
pub mod pointers;
pub mod screen;
pub mod surfaces;
pub mod window;

/// Little enum to make it clearer what some return values mean.
#[derive(Copy, Clone)]
enum Changed {
    Changed,
    Unchanged,
}

impl Changed {
    fn is_changed(self) -> bool {
        matches!(self, Changed::Changed)
    }
}
