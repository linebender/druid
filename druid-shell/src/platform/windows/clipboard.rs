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

//! Interactions with the system pasteboard on Windows.

use std::ffi::CString;
use std::mem;
use std::ptr;

use winapi::shared::minwindef::{FALSE, UINT};
use winapi::shared::ntdef::{CHAR, HANDLE, LPWSTR, WCHAR};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use winapi::um::winuser::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameA, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatA,
    SetClipboardData, CF_UNICODETEXT,
};

use super::util::{FromWide, ToWide};
use crate::clipboard::{ClipboardFormat, FormatId};

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, s: impl AsRef<str>) {
        let s = s.as_ref();
        let format: ClipboardFormat = s.into();
        self.put_formats(&[format])
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        with_clipboard(|| unsafe {
            EmptyClipboard();

            for format in formats {
                let handle = make_handle(&format);
                let format_id = match get_format_id(&format.identifier) {
                    Some(id) => id,
                    None => {
                        log::warn!("failed to register clipboard format {}", &format.identifier);
                        continue;
                    }
                };
                let result = SetClipboardData(format_id, handle);
                if result.is_null() {
                    log::warn!(
                        "failed to set clipboard for fmt {}, error: {}",
                        &format.identifier,
                        GetLastError()
                    );
                }
            }
        });
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        with_clipboard(|| unsafe {
            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_null() {
                None
            } else {
                let unic_str = GlobalLock(handle) as LPWSTR;
                let result = unic_str.from_wide();
                GlobalUnlock(handle);
                result
            }
        })
        .flatten()
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        with_clipboard(|| {
            let format_ids = formats
                .iter()
                .map(|id| get_format_id(id))
                .collect::<Vec<_>>();

            let first_match =
                iter_clipboard_types().find(|available| format_ids.contains(&Some(*available)));
            let format_idx =
                first_match.and_then(|id| format_ids.iter().position(|i| i == &Some(id)));
            format_idx.map(|idx| formats[idx])
        })
        .flatten()
    }

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        if format == ClipboardFormat::TEXT {
            return self.get_string().map(String::into_bytes);
        }

        with_clipboard(|| {
            let format_id = match get_format_id(&format) {
                Some(id) => id,
                None => {
                    log::warn!("failed to register clipboard format {}", &format);
                    return None;
                }
            };

            unsafe {
                if IsClipboardFormatAvailable(format_id) != 0 {
                    let handle = GetClipboardData(format_id);
                    let size = GlobalSize(handle);
                    let locked = GlobalLock(handle) as *const u8;
                    let mut dest = Vec::<u8>::with_capacity(size);
                    ptr::copy_nonoverlapping(locked, dest.as_mut_ptr(), size);
                    dest.set_len(size);
                    GlobalUnlock(handle);
                    Some(dest)
                } else {
                    None
                }
            }
        })
        .flatten()
    }

    pub fn available_type_names(&self) -> Vec<String> {
        with_clipboard(|| {
            iter_clipboard_types()
                .map(|id| format!("{}: {}", get_format_name(id), id))
                .collect()
        })
        .unwrap_or_default()
    }
}

fn with_clipboard<V>(f: impl FnOnce() -> V) -> Option<V> {
    unsafe {
        if OpenClipboard(ptr::null_mut()) == FALSE {
            return None;
        }

        let result = f();

        CloseClipboard();

        Some(result)
    }
}

unsafe fn make_handle(format: &ClipboardFormat) -> HANDLE {
    if format.identifier == ClipboardFormat::TEXT {
        let s = std::str::from_utf8_unchecked(&format.data);
        let wstr = s.to_wide();
        let handle = GlobalAlloc(GMEM_MOVEABLE, wstr.len() * mem::size_of::<WCHAR>());
        let locked = GlobalLock(handle) as LPWSTR;
        ptr::copy_nonoverlapping(wstr.as_ptr(), locked, wstr.len());
        GlobalUnlock(handle);
        handle
    } else {
        let handle = GlobalAlloc(GMEM_MOVEABLE, format.data.len() * mem::size_of::<CHAR>());
        let locked = GlobalLock(handle) as *mut u8;
        ptr::copy_nonoverlapping(format.data.as_ptr(), locked, format.data.len());
        GlobalUnlock(handle);
        handle
    }
}

fn get_format_id(format: FormatId) -> Option<UINT> {
    if let Some((id, _)) = STANDARD_FORMATS.iter().find(|(_, s)| s == &format) {
        return Some(*id);
    }
    match format {
        ClipboardFormat::TEXT => Some(CF_UNICODETEXT),
        other => register_identifier(other),
    }
}

fn register_identifier(ident: &str) -> Option<UINT> {
    let cstr = match CString::new(ident) {
        Ok(s) => s,
        Err(_) => {
            // granted this should happen _never_, but unwrap feels bad
            log::warn!("Null byte in clipboard identifier '{}'", ident);
            return None;
        }
    };
    unsafe {
        let pb_format = RegisterClipboardFormatA(cstr.as_ptr());
        if pb_format == 0 {
            let err = GetLastError();
            log::warn!(
                "failed to register clipboard format '{}'; error {}.",
                ident,
                err
            );
            return None;
        }
        Some(pb_format)
    }
}

fn iter_clipboard_types() -> impl Iterator<Item = UINT> {
    struct ClipboardTypeIter {
        last: UINT,
        done: bool,
    }

    impl Iterator for ClipboardTypeIter {
        type Item = UINT;

        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            unsafe {
                let nxt = EnumClipboardFormats(self.last);
                match nxt {
                    0 => {
                        self.done = true;
                        match GetLastError() {
                            ERROR_SUCCESS => (),
                            other => {
                                log::error!("iterating clipboard formats failed, error={}", other)
                            }
                        }
                        None
                    }
                    nxt => {
                        self.last = nxt;
                        Some(nxt)
                    }
                }
            }
        }
    }
    ClipboardTypeIter {
        last: 0,
        done: false,
    }
}

fn get_format_name(format: UINT) -> String {
    if let Some(name) = get_standard_format_name(format) {
        return name.to_owned();
    }

    const BUF_SIZE: usize = 64;
    unsafe {
        let mut buffer: [CHAR; BUF_SIZE] = [0; BUF_SIZE];
        let result = GetClipboardFormatNameA(format, buffer.as_mut_ptr(), BUF_SIZE as i32);
        if result > 0 {
            let len = result as usize;
            let lpstr = std::slice::from_raw_parts(buffer.as_ptr() as *const u8, len);
            String::from_utf8_lossy(&lpstr).into_owned()
        } else {
            let err = GetLastError();
            if err == 87 {
                String::from("Unknown Format")
            } else {
                log::warn!(
                    "error getting clipboard format name for format {}, errno {}",
                    format,
                    err
                );
                String::new()
            }
        }
    }
}

// https://docs.microsoft.com/en-ca/windows/win32/dataxchg/standard-clipboard-formats
static STANDARD_FORMATS: &[(UINT, &str)] = &[
    (1, "CF_TEXT"),
    (2, "CF_BITMAP"),
    (3, "CF_METAFILEPICT"),
    (4, "CF_SYLK"),
    (5, "CF_DIF"),
    (6, "CF_TIFF"),
    (7, "CF_OEMTEXT"),
    (8, "CF_DIB"),
    (9, "CF_PALETTE"),
    (10, "CF_PENDATA"),
    (11, "CF_RIFF"),
    (12, "CF_WAVE"),
    (13, "CF_UNICODETEXT"),
    (14, "CF_ENHMETAFILE"),
    (15, "CF_HDROP"),
    (16, "CF_LOCALE"),
    (17, "CF_DIBV5"),
    (0x0080, "CF_OWNERDISPLAY"),
    (0x0081, "CF_DSPTEXT"),
    (0x0082, "CF_DSPBITMAP"),
    (0x0083, "CF_DSPMETAFILEPICT"),
    (0x008E, "CF_DSPENHMETAFILE"),
    (0x0200, "CF_PRIVATEFIRST"),
    (0x02FF, "CF_PRIVATELAST"),
    (0x0300, "CF_GDIOBJFIRST"),
    (0x03FF, "CF_GDIOBJLAST"),
];

fn get_standard_format_name(format: UINT) -> Option<&'static str> {
    STANDARD_FORMATS
        .iter()
        .find(|(id, _)| *id == format)
        .map(|(_, s)| *s)
}
