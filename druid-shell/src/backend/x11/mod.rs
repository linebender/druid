// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! X11 implementation of `druid-shell`.

// TODO(x11/render_improvements): screen is currently flashing when resizing in perftest.
//     Might be related to the "sleep scheduler" in XWindow::render()?
// TODO(x11/render_improvements): double-buffering / present strategies / etc?

// # Notes on error handling in X11
//
// In XCB, errors are reported asynchronously by default, by sending them to the event
// loop. You can also request a synchronous error for a given call; we use this in
// window initialization, but otherwise we take the async route.
//
// When checking for X11 errors synchronously, there are two places where the error could
// happen. An error on the request means the connection is broken. There's no need for
// extra error context here, because the fact that the connection broke has nothing to do
// with what we're trying to do. An error on the reply means there was something wrong with
// the request, and so we add context. This convention is used throughout the x11 backend.

#[macro_use]
mod util;

pub mod application;
pub mod clipboard;
pub mod dialog;
pub mod error;
pub mod menu;
pub mod screen;
pub mod window;
