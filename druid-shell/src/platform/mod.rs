// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Platorm specific extensions.

#[cfg(any(
    doc,
    any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
))]
pub mod linux;

#[cfg(any(doc, target_os = "macos"))]
pub mod mac;
