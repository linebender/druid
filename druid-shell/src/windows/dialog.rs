// Copyright 2018 The xi-editor Authors.
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

//! File open/save dialogs, Windows implementation.

#![allow(non_upper_case_globals)]

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::LPWSTR;
use winapi::shared::windef::*;
use winapi::shared::wtypesbase::*;
use winapi::um::combaseapi::*;
use winapi::um::shobjidl::*;
use winapi::um::shobjidl_core::*;
use winapi::Interface;
use wio::com::ComPtr;

use std::ffi::OsString;
use std::ptr::null_mut;
use crate::util::{as_result, FromWide};
use crate::Error;

/// Type of file dialog.
pub enum FileDialogType {
    /// File open dialog.
    Open,
    /// File save dialog.
    Save,
}

/// Options for file dialog.
///
/// See documentation for
/// [_FILEOPENDIALOGOPTIONS](https://docs.microsoft.com/en-us/windows/desktop/api/shobjidl_core/ne-shobjidl_core-_fileopendialogoptions)
/// for more information on the winapi implementation.
#[derive(Default)]
pub struct FileDialogOptions(DWORD);

impl FileDialogOptions {
    /// Include system and hidden items.
    ///
    /// Maps to `FOS_FORCESHOWHIDDEN` in winapi.
    pub fn set_show_hidden(&mut self) {
        self.0 |= FOS_FORCESHOWHIDDEN;
    }

    // TODO: more options as needed
}

// TODO: remove these when they get added to winapi
DEFINE_GUID! {CLSID_FileOpenDialog,
0xDC1C5A9C, 0xE88A, 0x4DDE, 0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7}
DEFINE_GUID! {CLSID_FileSaveDialog,
0xC0B4E2F3, 0xBA21, 0x4773, 0x8D, 0xBA, 0x33, 0x5E, 0xC9, 0x46, 0xEB, 0x8B}

pub(crate) unsafe fn get_file_dialog_path(
    hwnd_owner: HWND,
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Result<OsString, Error> {
    let mut pfd: *mut IFileDialog = null_mut();
    let (class, id) = match ty {
        FileDialogType::Open => (&CLSID_FileOpenDialog, IFileOpenDialog::uuidof()),
        FileDialogType::Save => (&CLSID_FileSaveDialog, IFileSaveDialog::uuidof()),
    };
    as_result(CoCreateInstance(
        class,
        null_mut(),
        CLSCTX_INPROC_SERVER,
        &id,
        &mut pfd as *mut *mut IFileDialog as *mut LPVOID,
    ))?;
    let file_dialog = ComPtr::from_raw(pfd);
    as_result(file_dialog.SetOptions(options.0))?;
    as_result(file_dialog.Show(hwnd_owner))?;
    let mut result_ptr: *mut IShellItem = null_mut();
    as_result(file_dialog.GetResult(&mut result_ptr))?;
    let shell_item = ComPtr::from_raw(result_ptr);
    let mut display_name: LPWSTR = null_mut();
    as_result(shell_item.GetDisplayName(SIGDN_FILESYSPATH, &mut display_name))?;
    let filename = display_name.to_os_string();
    CoTaskMemFree(display_name as LPVOID);

    Ok(filename)
}
