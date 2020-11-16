// Copyright 2017 The Druid Authors.
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

use std::ffi::{CString, OsStr, OsString};
use std::mem;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::ptr;
use std::slice;

use lazy_static::lazy_static;
use winapi::shared::minwindef::{BOOL, HMODULE, UINT};
use winapi::shared::ntdef::{HRESULT, LPWSTR};
use winapi::shared::windef::{HMONITOR, HWND, RECT};
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::fileapi::{CreateFileA, GetFileType, OPEN_EXISTING};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::libloaderapi::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use winapi::um::processenv::{GetStdHandle, SetStdHandle};
use winapi::um::shellscalingapi::{MONITOR_DPI_TYPE, PROCESS_DPI_AWARENESS};
use winapi::um::winbase::{FILE_TYPE_UNKNOWN, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::wincon::{AttachConsole, ATTACH_PARENT_PROCESS};
use winapi::um::winnt::{FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};

use crate::kurbo::Rect;
use crate::region::Region;
use crate::scale::{Scalable, Scale};

use super::error::Error;

pub fn as_result(hr: HRESULT) -> Result<(), Error> {
    if SUCCEEDED(hr) {
        Ok(())
    } else {
        Err(Error::Hr(hr))
    }
}

impl From<HRESULT> for Error {
    fn from(hr: HRESULT) -> Error {
        Error::Hr(hr)
    }
}

pub trait ToWide {
    fn to_wide_sized(&self) -> Vec<u16>;
    fn to_wide(&self) -> Vec<u16>;
}

impl<T> ToWide for T
where
    T: AsRef<OsStr>,
{
    fn to_wide_sized(&self) -> Vec<u16> {
        self.as_ref().encode_wide().collect()
    }
    fn to_wide(&self) -> Vec<u16> {
        self.as_ref().encode_wide().chain(Some(0)).collect()
    }
}

pub trait FromWide {
    fn to_u16_slice(&self) -> &[u16];

    fn to_os_string(&self) -> OsString {
        OsStringExt::from_wide(self.to_u16_slice())
    }

    fn from_wide(&self) -> Option<String> {
        String::from_utf16(self.to_u16_slice()).ok()
    }
}

impl FromWide for LPWSTR {
    fn to_u16_slice(&self) -> &[u16] {
        unsafe {
            let mut len = 0;
            while *self.offset(len) != 0 {
                len += 1;
            }
            slice::from_raw_parts(*self, len as usize)
        }
    }
}

impl FromWide for [u16] {
    fn to_u16_slice(&self) -> &[u16] {
        self
    }
}

/// Converts a `Rect` to a winapi `RECT`.
#[inline]
pub(crate) fn rect_to_recti(rect: Rect) -> RECT {
    RECT {
        left: rect.x0 as i32,
        top: rect.y0 as i32,
        right: rect.x1 as i32,
        bottom: rect.y1 as i32,
    }
}

/// Converts a winapi `RECT` to a `Rect`.
#[inline]
pub(crate) fn recti_to_rect(rect: RECT) -> Rect {
    Rect::new(
        rect.left as f64,
        rect.top as f64,
        rect.right as f64,
        rect.bottom as f64,
    )
}

/// Converts a `Region` into a vec of winapi `RECT`, with a scaling applied.
/// If necessary, the rectangles are rounded to the nearest pixel border.
pub(crate) fn region_to_rectis(region: &Region, scale: Scale) -> Vec<RECT> {
    region
        .rects()
        .iter()
        .map(|r| rect_to_recti(r.to_px(scale).round()))
        .collect()
}

// Types for functions we want to load, which are only supported on newer windows versions
// from user32.dll
type GetDpiForSystem = unsafe extern "system" fn() -> UINT;
type GetDpiForWindow = unsafe extern "system" fn(HWND) -> UINT;
type SetProcessDpiAwarenessContext =
    unsafe extern "system" fn(winapi::shared::windef::DPI_AWARENESS_CONTEXT) -> BOOL;
type GetSystemMetricsForDpi =
    unsafe extern "system" fn(winapi::ctypes::c_int, UINT) -> winapi::ctypes::c_int;
// from shcore.dll
type GetDpiForMonitor = unsafe extern "system" fn(HMONITOR, MONITOR_DPI_TYPE, *mut UINT, *mut UINT);
type SetProcessDpiAwareness = unsafe extern "system" fn(PROCESS_DPI_AWARENESS) -> HRESULT;

#[allow(non_snake_case)] // For member fields
pub struct OptionalFunctions {
    pub GetDpiForSystem: Option<GetDpiForSystem>,
    pub GetDpiForWindow: Option<GetDpiForWindow>,
    pub SetProcessDpiAwarenessContext: Option<SetProcessDpiAwarenessContext>,
    pub GetDpiForMonitor: Option<GetDpiForMonitor>,
    pub SetProcessDpiAwareness: Option<SetProcessDpiAwareness>,
    pub GetSystemMetricsForDpi: Option<GetSystemMetricsForDpi>,
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
                log::info!(
                    "Could not load `{}`. Windows {} or later is needed",
                    name,
                    $min_windows_version
                );
            } else {
                let function = unsafe { mem::transmute::<_, $function>(function_ptr) };
                $function = Some(function);
            }
        }};
    }

    fn load_library(name: &str) -> HMODULE {
        let encoded_name = name.to_wide();

        // If we already have loaded the library (somewhere else in the process) we don't need to
        // call LoadLibrary again
        let library = unsafe { GetModuleHandleW(encoded_name.as_ptr()) };
        if !library.is_null() {
            return library;
        }

        unsafe { LoadLibraryW(encoded_name.as_ptr()) }
    }

    let shcore = load_library("shcore.dll");
    let user32 = load_library("user32.dll");

    let mut GetDpiForSystem = None;
    let mut GetDpiForMonitor = None;
    let mut GetDpiForWindow = None;
    let mut SetProcessDpiAwarenessContext = None;
    let mut SetProcessDpiAwareness = None;
    let mut GetSystemMetricsForDpi = None;

    if shcore.is_null() {
        log::info!("No shcore.dll");
    } else {
        load_function!(shcore, SetProcessDpiAwareness, "8.1");
        load_function!(shcore, GetDpiForMonitor, "8.1");
    }

    if user32.is_null() {
        log::info!("No user32.dll");
    } else {
        load_function!(user32, GetDpiForSystem, "10");
        load_function!(user32, GetDpiForWindow, "10");
        load_function!(user32, SetProcessDpiAwarenessContext, "10");
        load_function!(user32, GetSystemMetricsForDpi, "10");
    }

    OptionalFunctions {
        GetDpiForSystem,
        GetDpiForWindow,
        SetProcessDpiAwarenessContext,
        GetDpiForMonitor,
        SetProcessDpiAwareness,
        GetSystemMetricsForDpi,
    }
}

lazy_static! {
    pub static ref OPTIONAL_FUNCTIONS: OptionalFunctions = load_optional_functions();
}

pub(crate) const CLASS_NAME: &str = "druid";

/// Convenience macro for defining accelerator tables.
#[macro_export]
macro_rules! accel {
    ( $( $fVirt:expr, $key:expr, $cmd:expr, )* ) => {
        [
            $(
                ACCEL { fVirt: $fVirt | FVIRTKEY, key: $key as WORD, cmd: $cmd as WORD },
            )*
        ]
    }
}

/// Attach the process to the console of the parent process. This allows xi-win to
/// correctly print to a console when run from powershell or cmd.
/// If no console is available, allocate a new console.
pub(crate) fn attach_console() {
    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout != INVALID_HANDLE_VALUE && GetFileType(stdout) != FILE_TYPE_UNKNOWN {
            // We already have a perfectly valid stdout and must not attach.
            // This happens, for example in MingW consoles like git bash.
            return;
        }

        if AttachConsole(ATTACH_PARENT_PROCESS) > 0 {
            let name = CString::new("CONOUT$").unwrap();
            let chnd = CreateFileA(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );

            if chnd == INVALID_HANDLE_VALUE {
                // CreateFileA failed.
                return;
            }

            SetStdHandle(STD_OUTPUT_HANDLE, chnd);
            SetStdHandle(STD_ERROR_HANDLE, chnd);
        }
    }
}
