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

#[cfg(target_os = "windows")]
use winapi::shared::winerror::HRESULT;

/// Error codes. At the moment, this is little more than HRESULT, but that
/// might change.
pub enum Error {
    Other(&'static str),
    #[cfg(target_os = "windows")]
    Hr(HRESULT),
    // Maybe include the full error from the direct2d crate.
    #[cfg(target_os = "windows")]
    D2Error,
    /// A function is available on newer version of windows.
    #[cfg(target_os = "windows")]
    OldWindows,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::Other(s) => write!(f, "{}", s),
            #[cfg(target_os = "windows")]
            Error::Hr(hr) => write!(f, "HRESULT 0x{:x}", hr),
            #[cfg(target_os = "windows")]
            Error::D2Error => write!(f, "Direct2D error"),
            #[cfg(target_os = "windows")]
            Error::OldWindows => write!(f, "Attempted newer API on older Windows"),
        }
    }
}
