// Copyright 2018 The Druid Authors.
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
//!
//! For more information about how windows handles file dialogs, see
//! documentation for [_FILEOPENDIALOGOPTIONS] and [SetFileTypes].
//!
//! [_FILEOPENDIALOGOPTIONS]: https://docs.microsoft.com/en-us/windows/desktop/api/shobjidl_core/ne-shobjidl_core-_fileopendialogoptions
//! [SetFileTypes]: https://docs.microsoft.com/en-ca/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifiledialog-setfiletypes

#![allow(non_upper_case_globals)]

use std::convert::TryInto;
use std::ffi::OsString;
use std::ptr::null_mut;

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::LPWSTR;
use winapi::shared::windef::*;
use winapi::shared::wtypesbase::*;
use winapi::um::combaseapi::*;
use winapi::um::shobjidl::*;
use winapi::um::shobjidl_core::*;
use winapi::um::shtypes::COMDLG_FILTERSPEC;
use winapi::{Interface, DEFINE_GUID};
use wio::com::ComPtr;

use super::error::Error;
use super::util::{as_result, FromWide, ToWide};
use crate::dialog::{FileDialogOptions, FileDialogType, FileSpec};

// TODO: remove these when they get added to winapi
DEFINE_GUID! {CLSID_FileOpenDialog,
0xDC1C_5A9C, 0xE88A, 0x4DDE, 0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7}
DEFINE_GUID! {CLSID_FileSaveDialog,
0xC0B4_E2F3, 0xBA21, 0x4773, 0x8D, 0xBA, 0x33, 0x5E, 0xC9, 0x46, 0xEB, 0x8B}

/// For each item in `spec`, returns a pair of utf16 strings representing the name
/// and the filter spec.
///
/// As an example: given the name `"Markdown Document"`, and the extensions
/// `&["md", "mdown", "markdown"]`, this will return
/// `("Markdown Document (*.md; *.mdown; *.markdown)", "*.md;*.mdown;*.markdown")`.
///
/// The first of these is displayed to the user, and the second is used to match the path.
unsafe fn make_wstrs(spec: &FileSpec) -> (Vec<u16>, Vec<u16>) {
    let exts = spec
        .extensions
        .iter()
        .map(|s| normalize_extension(*s))
        .collect::<Vec<_>>();
    let name = format!("{} ({})", spec.name, exts.as_slice().join("; ")).to_wide();
    let extensions = exts.as_slice().join(";").to_wide();
    (name, extensions)
}

/// add preceding *., trimming preceding *. that might've been included by the user.
fn normalize_extension(ext: &str) -> String {
    format!("*.{}", ext.trim_start_matches('*').trim_start_matches('.'))
}

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

    // set options
    let mut flags: DWORD = 0;
    if options.show_hidden {
        flags |= FOS_FORCESHOWHIDDEN;
    }
    if options.select_directories {
        flags |= FOS_PICKFOLDERS;
    }
    if options.multi_selection {
        flags |= FOS_ALLOWMULTISELECT;
    }

    // - allowed filetypes

    // this is a vec of vec<u16>, e.g wide strings. this memory needs to be live
    // until `Show` returns.
    let spec = options.allowed_types.as_ref().map(|allowed_types| {
        allowed_types
            .iter()
            .map(|t| make_wstrs(t))
            .collect::<Vec<_>>()
    });

    // this is a vector of structs whose members point into the vector above.
    // this has to outlive that vector.
    let raw_spec = spec.as_ref().map(|buf_pairs| {
        buf_pairs
            .iter()
            .map(|(name, ext)| COMDLG_FILTERSPEC {
                pszName: name.as_ptr(),
                pszSpec: ext.as_ptr(),
            })
            .collect::<Vec<_>>()
    });

    // Having Some in raw_spec means that we can safely unwrap options.allowed_types.
    // Be careful when moving the unwraps out of this block or changing how raw_spec is made.
    if let Some(spec) = &raw_spec {
        // We also want to make sure that options.allowed_types wasn't Some(zero-length vector)
        if !spec.is_empty() {
            as_result(file_dialog.SetFileTypes(spec.len() as u32, spec.as_ptr()))?;

            let index = match (&options.default_type, &options.allowed_types) {
                (Some(typ), Some(typs)) => typs.iter().position(|t| t == typ).unwrap_or(0),
                _ => 0,
            };

            let default_allowed_type = options.allowed_types.as_ref().unwrap().get(index).unwrap();
            let default_extension = default_allowed_type.extensions.first().unwrap_or(&"");

            as_result(file_dialog.SetFileTypeIndex((index + 1).try_into().unwrap()))?;
            as_result(file_dialog.SetDefaultExtension(default_extension.to_wide().as_ptr()))?;
        }
    }

    as_result(file_dialog.SetOptions(flags))?;

    // show the dialog
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
