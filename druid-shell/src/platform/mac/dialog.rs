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

use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSArray, NSAutoreleasePool, NSInteger, NSURL};
use objc::{class, msg_send, sel, sel_impl};

use super::util::{from_nsstring, make_nsstring};
use crate::dialog::{FileDialogOptions, FileDialogType};

pub(crate) type NSModalResponse = NSInteger;
const NSModalResponseOK: NSInteger = 1;
const NSModalResponseCancel: NSInteger = 0;

pub(crate) unsafe fn get_path(panel: id, result: NSModalResponse) -> Option<OsString> {
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

#[allow(clippy::cognitive_complexity)]
pub(crate) unsafe fn build_panel(ty: FileDialogType, mut options: FileDialogOptions) -> id {
    let panel: id = match ty {
        FileDialogType::Open => msg_send![class!(NSOpenPanel), openPanel],
        FileDialogType::Save => msg_send![class!(NSSavePanel), savePanel],
    };

    // Enable the user to choose whether file extensions are hidden in the dialog.
    // This defaults to off, but is recommended to be turned on by Apple.
    // https://developer.apple.com/design/human-interface-guidelines/macos/user-interaction/file-handling/
    let () = msg_send![panel, setCanSelectHiddenExtension: YES];

    // Set open dialog specific options
    // NSOpenPanel inherits from NSSavePanel and thus has more options.
    let mut set_type_filter = true;
    if let FileDialogType::Open = ty {
        if options.select_directories {
            let () = msg_send![panel, setCanChooseDirectories: YES];
            // Disable the selection of files in directory selection mode,
            // because other platforms like Windows have no support for it,
            // and expecting it to work will lead to buggy cross-platform behavior.
            let () = msg_send![panel, setCanChooseFiles: NO];
            // File filters are used by macOS to determine which paths are packages.
            // If we are going to treat packages as directories, then file filters
            // need to be disabled. Otherwise macOS will allow picking of regular files
            // that match the filters as if they were directories too.
            set_type_filter = !options.packages_as_directories;
        }
        if options.multi_selection {
            let () = msg_send![panel, setAllowsMultipleSelection: YES];
        }
    }

    // Set universal options
    if options.packages_as_directories {
        let () = msg_send![panel, setTreatsFilePackagesAsDirectories: YES];
    }

    if options.show_hidden {
        let () = msg_send![panel, setShowsHiddenFiles: YES];
    }

    if let Some(default_name) = &options.default_name {
        let () = msg_send![panel, setNameFieldStringValue: make_nsstring(default_name)];
    }

    if let Some(name_label) = &options.name_label {
        let () = msg_send![panel, setNameFieldLabel: make_nsstring(name_label)];
    }

    if let Some(title) = &options.title {
        let () = msg_send![panel, setTitle: make_nsstring(title)];
    }

    if let Some(text) = &options.button_text {
        let () = msg_send![panel, setPrompt: make_nsstring(text)];
    }

    if let Some(path) = &options.starting_directory {
        if let Some(path) = path.to_str() {
            let url = NSURL::alloc(nil)
                .initFileURLWithPath_isDirectory_(make_nsstring(path), YES)
                .autorelease();
            let () = msg_send![panel, setDirectoryURL: url];
        }
    }

    if set_type_filter {
        // If a default type was specified, then we must reorder the allowed types,
        // because there's no way to specify the default type other than having it be first.
        if let Some(dt) = &options.default_type {
            let mut present = false;
            if let Some(allowed_types) = options.allowed_types.as_mut() {
                if let Some(idx) = allowed_types.iter().position(|t| t == dt) {
                    present = true;
                    allowed_types.swap(idx, 0);
                }
            }
            if !present {
                log::warn!("The default type {:?} is not present in allowed types.", dt);
            }
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
    }
    panel
}
