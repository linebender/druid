// Copyright 2019 The Druid Authors.
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

//! GTK platform errors.

use std::fmt;

use glib::{BoolError, Error as GLibError};

/// GTK platform errors.
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
            Error::Error(err) => write!(f, "GTK Error: {}", err),
            Error::BoolError(err) => write!(f, "GTK BoolError: {}", err),
        }
    }
}

impl std::error::Error for Error {}
