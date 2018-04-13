// Copyright 2017 Google Inc. All rights reserved.
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

//! Various utilities for working with windows. Includes utilities for converting between Windows 
//! and Rust types, including strings. 
//! Also includes some code to dynamically load functions at runtime. This is needed for functions
//! which are only supported on certain versions of windows.

use std::ffi::{OsStr, CString};
use std::os::windows::ffi::OsStrExt;
use std::slice;
use std::mem;

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::*;
use winapi::shared::windef::*;
use winapi::um::libloaderapi::*;
use winapi::um::shellscalingapi::*;

use direct2d::render_target::DrawTextOption;

#[derive(Debug)]
/// Error codes. At the moment, this is little more than HRESULT, but that
/// might change.
pub enum Error {
    Null,
    Hr(HRESULT),
    // Maybe include the full error from the direct2d crate.
    D2Error,
}

/*
pub fn as_result(hr: HRESULT) -> Result<(), Error> {
    match hr {
        S_OK => Ok(()),
        _ => Err(Error::Hr(hr)),
    }
}
*/

impl From<HRESULT> for Error {
    fn from(hr: HRESULT) -> Error {
        Error::Hr(hr)
    }
}

pub trait ToWide {
    fn to_wide_sized(&self) -> Vec<u16>;
    fn to_wide(&self) -> Vec<u16>;
}

impl<T> ToWide for T where T: AsRef<OsStr> {
    fn to_wide_sized(&self) -> Vec<u16> {
        self.as_ref().encode_wide().collect()
    }
    fn to_wide(&self) -> Vec<u16> {
        self.as_ref().encode_wide().chain(Some(0)).collect()
    }
}

pub trait FromWide {
    fn from_wide(&self) -> Option<String>;
}

impl FromWide for LPWSTR {
    fn from_wide(&self) -> Option<String> {
        unsafe {
            let mut len = 0;
            while *self.offset(len) != 0 {
                len += 1;
            }
            slice::from_raw_parts(*self, len as usize).from_wide()
        }
    }
}

impl FromWide for [u16] {
    fn from_wide(&self) -> Option<String> {
        String::from_utf16(self).ok()
    }
}

// Types for functions we want to load, which are only supported on newer windows versions
// from shcore.dll
type GetDpiForSystem = unsafe extern "system" fn() -> UINT;
type GetDpiForMonitor = unsafe extern "system" fn(HMONITOR, MONITOR_DPI_TYPE, *mut UINT, *mut UINT);
// from user32.dll
type SetProcessDpiAwareness = unsafe extern "system" fn(PROCESS_DPI_AWARENESS) -> HRESULT;

#[allow(non_snake_case)] // For member fields
pub struct OptionalFunctions {
    pub GetDpiForSystem: Option<GetDpiForSystem>,
    pub GetDpiForMonitor: Option<GetDpiForMonitor>,
    pub SetProcessDpiAwareness: Option<SetProcessDpiAwareness>,
}

#[allow(non_snake_case)] // For local variables
fn load_optional_functions() -> OptionalFunctions { 
    // Tries to load $function from $lib. $function should be one of the types defined just before 
    // `load_optional_functions`. This sets the corresponding local field to `Some(function pointer)`
    // if it manages to load the function.
    macro_rules! load_function {
        ($lib: expr, $function: ident, $min_windows_version: expr) => {{
            let name = stringify!($function);

            // The `unwrap` is fine, because we only call this macro for winapi functions, which
            // have simple ascii-only names
            let cstr = CString::new(name).unwrap();

            let function_ptr = unsafe { GetProcAddress($lib, cstr.as_ptr()) };

            if function_ptr.is_null() {
                println!(
                    "Could not load `{}`. Windows {} or later is needed",
                    name, $min_windows_version
                );
            } else {
                let function = unsafe { mem::transmute::<_, $function>(function_ptr) };
                $function = Some(function);
            }
        }};
    }

    fn load_library(name: &str) -> HMODULE {
        let encoded_name = name.to_wide();

        // If we allready have loaded the library (somewhere else in the process) we don't need to
        // call LoadLibrary again
        let library = unsafe { GetModuleHandleW(encoded_name.as_ptr()) };
        if !library.is_null() {
            return library;
        }

        let library = unsafe { LoadLibraryW(encoded_name.as_ptr()) };
        return library;
    }

    let shcore = load_library("shcore.dll");
    let user32 = load_library("user32.dll");

    let mut GetDpiForSystem = None;
    let mut GetDpiForMonitor = None;
    let mut SetProcessDpiAwareness = None;

    if shcore.is_null() {
        println!("No shcore.dll");
    } else {
        load_function!(shcore, SetProcessDpiAwareness, "8.1");
        load_function!(shcore, GetDpiForMonitor, "8.1");
    }

    if user32.is_null() {
        println!("No user32.dll");
    } else {
        load_function!(user32, GetDpiForSystem, "10");
    }

    OptionalFunctions {
        GetDpiForSystem,
        GetDpiForMonitor,
        SetProcessDpiAwareness,
    }
}

lazy_static! {
    pub static ref OPTIONAL_FUNCTIONS: OptionalFunctions = load_optional_functions();
}

/// Initialize the app. At the moment, this is mostly needed for hi-dpi.
pub fn init() {
    if let Some(func) = OPTIONAL_FUNCTIONS.SetProcessDpiAwareness {
        // This function is only supported on windows 10
        unsafe {
            func(PROCESS_SYSTEM_DPI_AWARE); // TODO: per monitor (much harder)
        }
    }
}

/// Determine a suitable default set of text options. Enables color fonts
/// on systems that are capable of them (8.1 and above).
pub fn default_text_options() -> &'static [DrawTextOption] {
    // This is an arbitrary optional function that is 8.1 and above.
    if OPTIONAL_FUNCTIONS.SetProcessDpiAwareness.is_some() {
        &[DrawTextOption::EnableColorFont]
    } else {
        &[]
    }
}
