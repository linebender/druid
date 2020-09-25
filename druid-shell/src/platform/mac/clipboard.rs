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

//! Interactions with the system pasteboard on macOS.

use cocoa::appkit::NSPasteboardTypeString;
use cocoa::base::{id, nil, BOOL, YES};
use cocoa::foundation::{NSArray, NSInteger, NSUInteger};
use objc::{class, msg_send, sel, sel_impl};

use super::util;
use crate::clipboard::{ClipboardFormat, FormatId};

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, s: impl AsRef<str>) {
        let s = s.as_ref();
        put_string_impl(s);

        fn put_string_impl(s: &str) {
            unsafe {
                let nsstring = util::make_nsstring(s);
                let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
                let _: NSInteger = msg_send![pasteboard, clearContents];
                let result: BOOL =
                    msg_send![pasteboard, setString: nsstring forType: NSPasteboardTypeString];
                if result != YES {
                    log::warn!("failed to set clipboard");
                }
            }
        }
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        unsafe {
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let _: NSInteger = msg_send![pasteboard, clearContents];
            let idents = formats
                .iter()
                .map(|f| util::make_nsstring(f.identifier))
                .collect::<Vec<_>>();
            let array = NSArray::arrayWithObjects(nil, &idents);
            let _: NSInteger = msg_send![pasteboard, declareTypes: array owner: nil];
            for (i, format) in formats.iter().enumerate() {
                let data = util::make_nsdata(&format.data);
                let data_type = idents[i];
                let result: BOOL = msg_send![pasteboard, setData: data forType: data_type];
                if result != YES {
                    log::warn!(
                        "failed to set clipboard contents for type '{}'",
                        format.identifier
                    );
                }
            }
        }
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        unsafe {
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let contents: id = msg_send![pasteboard, stringForType: NSPasteboardTypeString];
            if contents.is_null() {
                None
            } else {
                Some(util::from_nsstring(contents))
            }
        }
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        unsafe {
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let nsstrings = formats
                .iter()
                .map(|s| util::make_nsstring(s))
                .collect::<Vec<_>>();
            let array = NSArray::arrayWithObjects(nil, &nsstrings);
            let available: id = msg_send![pasteboard, availableTypeFromArray: array];
            if available.is_null() {
                None
            } else {
                let idx: NSUInteger = msg_send![array, indexOfObject: available];
                let idx = idx as usize;
                if idx > formats.len() {
                    log::error!("clipboard object not found");
                    None
                } else {
                    Some(formats[idx])
                }
            }
        }
    }

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    pub fn get_format(&self, fmt: FormatId) -> Option<Vec<u8>> {
        unsafe {
            let pb_type = util::make_nsstring(fmt);
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let data: id = msg_send![pasteboard, dataForType: pb_type];
            if !data.is_null() {
                let data = util::from_nsdata(data);
                Some(data)
            } else {
                None
            }
        }
    }

    pub fn available_type_names(&self) -> Vec<String> {
        unsafe {
            let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
            let types: id = msg_send![pasteboard, types];
            let types_len = types.count() as usize;
            (0..types_len)
                .map(|i| util::from_nsstring(types.objectAtIndex(i as NSUInteger)))
                .collect()
        }
    }
}
