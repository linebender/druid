// Copyright 2020 The xi-editor Authors.
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

#[derive(Debug, Clone)]
pub enum Error {
    // Generic error
    Generic(String),
    // TODO: Replace String with xcb::ConnError once that gets Clone support
    ConnectionError(String),
    // Runtime borrow failure
    BorrowError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Generic(msg) => write!(f, "Error: {}", msg),
            Error::ConnectionError(err) => write!(f, "Connection error: {}", err),
            Error::BorrowError(msg) => write!(f, "Failed to borrow: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
