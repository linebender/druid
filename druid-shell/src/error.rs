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

/// Error codes. At the moment, this is little more than HRESULT, but that
/// might change.
#[derive(Debug, Clone)]
pub enum Error {
    ApplicationAlreadyExists,
    Other(&'static str),
    Platform(platform::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::ApplicationAlreadyExists => {
                write!(f, "An application instance has already been created.")
            }
            Error::Other(s) => write!(f, "{}", s),
            Error::Platform(p) => fmt::Display::fmt(&p, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<platform::Error> for Error {
    fn from(src: platform::Error) -> Error {
        Error::Platform(src)
    }
}
