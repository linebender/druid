// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Utilities, GTK specific.

pub(crate) fn assert_main_thread() {
    assert!(gtk::is_initialized_main_thread());
}
