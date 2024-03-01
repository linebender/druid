// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! GTK backend errors.

use std::fmt;

use gtk::glib::{BoolError, Error as GLibError};

/// GTK backend errors.
#[derive(Debug, Clone)]
pub enum Error {
    /// Generic GTK error.
    Error(GLibError),
    /// GTK error that has no information provided by GTK,
    /// but may have extra information provided by gtk-rs.
    BoolError(BoolError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Error(err) => write!(f, "GTK Error: {err}"),
            Error::BoolError(err) => write!(f, "GTK BoolError: {err}"),
        }
    }
}

impl std::error::Error for Error {}
