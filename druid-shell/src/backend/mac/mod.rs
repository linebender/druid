// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! macOS `druid-shell` backend.

#![allow(clippy::let_unit_value)]

pub mod appkit;
pub mod application;
pub mod clipboard;
pub mod dialog;
pub mod error;
mod keyboard;
pub mod menu;
pub mod screen;
pub mod text_input;
pub mod util;
pub mod window;
