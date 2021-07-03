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

//! Errors at the application shell level.

use std::fmt;
use std::ptr::{null, null_mut};

use winapi::shared::minwindef::{DWORD, HLOCAL};
use winapi::shared::ntdef::LPWSTR;
use winapi::shared::winerror::HRESULT;
use winapi::um::winbase::{
    FormatMessageW, LocalFree, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
    FORMAT_MESSAGE_IGNORE_INSERTS, FORMAT_MESSAGE_MAX_WIDTH_MASK,
};

use super::util::FromWide;

/// Windows platform error.
#[derive(Debug, Clone)]
pub enum Error {
    /// Windows error code.
    Hr(HRESULT),
    // Maybe include the full error from the direct2d crate.
    D2Error,
    /// A function is available on newer version of windows.
    OldWindows,
    /// The `hwnd` pointer was null.
    NullHwnd,
}

fn hresult_description(hr: HRESULT) -> Option<String> {
    unsafe {
        let mut message_buffer: LPWSTR = std::ptr::null_mut();
        let format_result = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_ALLOCATE_BUFFER
                | FORMAT_MESSAGE_IGNORE_INSERTS
                | FORMAT_MESSAGE_MAX_WIDTH_MASK,
            null(),
            hr as DWORD,
            0,
            &mut message_buffer as *mut LPWSTR as LPWSTR,
            0,
            null_mut(),
        );
        if format_result == 0 || message_buffer.is_null() {
            return None;
        }

        let result = message_buffer.from_wide();
        LocalFree(message_buffer as HLOCAL);
        result
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Hr(hr) => {
                write!(f, "HRESULT 0x{:x}", hr)?;
                if let Some(description) = hresult_description(*hr) {
                    write!(f, ": {}", description)?;
                }
                Ok(())
            }
            Error::D2Error => write!(f, "Direct2D error"),
            Error::OldWindows => write!(f, "Attempted newer API on older Windows"),
            Error::NullHwnd => write!(f, "Window handle is Null"),
        }
    }
}

impl std::error::Error for Error {}
