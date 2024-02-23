// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Errors at the application shell level.

use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Error {
    XError(Arc<x11rb::errors::ReplyError>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let Error::XError(e) = self;
        e.fmt(f)
    }
}

impl std::error::Error for Error {}

impl From<x11rb::x11_utils::X11Error> for Error {
    fn from(err: x11rb::x11_utils::X11Error) -> Error {
        Error::XError(Arc::new(x11rb::errors::ReplyError::X11Error(err)))
    }
}

impl From<x11rb::errors::ReplyError> for Error {
    fn from(err: x11rb::errors::ReplyError) -> Error {
        Error::XError(Arc::new(err))
    }
}
