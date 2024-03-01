// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Monitor and Screen information ignored for web.

use crate::screen::Monitor;

pub(crate) fn get_monitors() -> Vec<Monitor> {
    tracing::warn!("Screen::get_monitors() is not implemented for web.");
    Vec::new()
}
