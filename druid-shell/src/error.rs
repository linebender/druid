// Copyright 2019 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Errors at the application shell level.

use std::fmt;

use crate::platform::error as platform;

/// Shell errors.
#[derive(Debug, Clone)]
pub enum Error {
    /// The Application instance has already been created.
    ApplicationAlreadyExists,
    /// The window has already been destroyed.
    WindowDropped,
    /// Runtime borrow failure.
    BorrowError(BorrowError),
    /// Platform specific error.
    Platform(platform::Error),
    /// Other miscellaneous error.
    Other(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::ApplicationAlreadyExists => {
                write!(f, "An application instance has already been created.")
            }
            Error::WindowDropped => write!(f, "The window has already been destroyed."),
            Error::BorrowError(err) => fmt::Display::fmt(err, f),
            Error::Platform(err) => fmt::Display::fmt(err, f),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<platform::Error> for Error {
    fn from(src: platform::Error) -> Error {
        Error::Platform(src)
    }
}

/// Runtime borrow failure.
#[derive(Debug, Clone)]
pub struct BorrowError {
    location: &'static str,
    target: &'static str,
    mutable: bool,
}

impl BorrowError {
    pub fn new(location: &'static str, target: &'static str, mutable: bool) -> BorrowError {
        BorrowError {
            location,
            target,
            mutable,
        }
    }
}

impl fmt::Display for BorrowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.mutable {
            // Mutable borrow fails when any borrow exists
            write!(
                f,
                "{} was already borrowed in {}",
                self.target, self.location
            )
        } else {
            // Regular borrow fails when a mutable borrow exists
            write!(
                f,
                "{} was already mutably borrowed in {}",
                self.target, self.location
            )
        }
    }
}

impl std::error::Error for BorrowError {}

impl From<BorrowError> for Error {
    fn from(src: BorrowError) -> Error {
        Error::BorrowError(src)
    }
}
