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

//! Windows implementation of features at the application scope.

use std::ptr;

use winapi::shared::minwindef::FALSE;
use winapi::shared::minwindef::UINT;
use winapi::shared::ntdef::{LPWSTR, WCHAR};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use winapi::um::winuser::*;

use crate::clipboard::ClipboardItem;
use crate::util::{FromWide, ToWide};

pub struct Application;

impl Application {
    pub fn quit() {
        crate::runloop::request_quit();
    }

    /// Returns the contents of the clipboard, if any.
    pub fn get_clipboard_contents() -> Option<ClipboardItem> {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == FALSE {
                return None;
            }

            let result = get_clipboard_impl();
            CloseClipboard();
            result
        }
    }

    pub fn set_clipboard_contents(item: ClipboardItem) {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == FALSE {
                return;
            }
            EmptyClipboard();
            match item {
                ClipboardItem::Text(string) => {
                    let wstr = string.to_wide();
                    let wstr_copy =
                        GlobalAlloc(GMEM_MOVEABLE, wstr.len() * std::mem::size_of::<WCHAR>());
                    let locked = GlobalLock(wstr_copy) as LPWSTR;
                    ptr::copy_nonoverlapping(wstr.as_ptr(), locked, wstr.len());
                    GlobalUnlock(wstr_copy);
                    SetClipboardData(CF_UNICODETEXT, wstr_copy);
                }
                other => log::warn!("unhandled clipboard data {:?}", other),
            }
            CloseClipboard();
        }
    }
}

#[allow(clippy::single_match)] // we will support more types 'soon'
unsafe fn get_clipboard_impl() -> Option<ClipboardItem> {
    for format in iter_clipboard_types() {
        match format {
            CF_UNICODETEXT => return get_unicode_text(),
            _ => (),
        }
    }
    None
}

unsafe fn get_unicode_text() -> Option<ClipboardItem> {
    let handle = GetClipboardData(CF_UNICODETEXT);
    let result = if handle.is_null() {
        let unic_str = GlobalLock(handle) as LPWSTR;
        let result = unic_str.from_wide();
        GlobalUnlock(handle);
        result
    } else {
        None
    };
    result.map(Into::into)
}

pub(crate) fn iter_clipboard_types() -> impl Iterator<Item = UINT> {
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
