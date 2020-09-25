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

//! File open/save dialogs, macOS implementation.

#![allow(non_upper_case_globals)]

use std::ffi::OsString;

use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSArray, NSInteger};
use objc::{class, msg_send, sel, sel_impl};

use super::util::{from_nsstring, make_nsstring};
use crate::dialog::{FileDialogOptions, FileDialogType};

const NSModalResponseOK: NSInteger = 1;
const NSModalResponseCancel: NSInteger = 0;

pub(crate) fn get_file_dialog_path(
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Option<OsString> {
    unsafe {
        let panel: id = match ty {
            FileDialogType::Open => msg_send![class!(NSOpenPanel), openPanel],
            FileDialogType::Save => msg_send![class!(NSSavePanel), savePanel],
        };

        // set options
        if options.show_hidden {
            let () = msg_send![panel, setShowsHiddenFiles: YES];
        }

        if options.select_directories {
            let () = msg_send![panel, setCanChooseDirectories: YES];
        }

        if options.multi_selection {
            let () = msg_send![panel, setAllowsMultipleSelection: YES];
        }

        // A vector of NSStrings. this must outlive `nsarray_allowed_types`.
        let allowed_types = options.allowed_types.as_ref().map(|specs| {
            specs
                .iter()
                .flat_map(|spec| spec.extensions.iter().map(|s| make_nsstring(s)))
                .collect::<Vec<_>>()
        });

        let nsarray_allowed_types = allowed_types
            .as_ref()
            .map(|types| NSArray::arrayWithObjects(nil, types.as_slice()));
        if let Some(nsarray) = nsarray_allowed_types {
            let () = msg_send![panel, setAllowedFileTypes: nsarray];
        }

        let result: NSInteger = msg_send![panel, runModal];
        match result {
            NSModalResponseOK => {
                let url: id = msg_send![panel, URL];
                let path: id = msg_send![url, path];
                let path: OsString = from_nsstring(path).into();
                Some(path)
            }
            NSModalResponseCancel => None,
            _ => unreachable!(),
        }
    }
}
