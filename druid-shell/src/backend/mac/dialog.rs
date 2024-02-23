// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! File open/save dialogs, macOS implementation.

#![allow(non_upper_case_globals, clippy::upper_case_acronyms)]

use std::ffi::OsString;

use cocoa::appkit::NSView;
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSArray, NSAutoreleasePool, NSInteger, NSPoint, NSRect, NSSize, NSURL};
use objc::{class, msg_send, sel, sel_impl};

use super::util::{from_nsstring, make_nsstring};
use crate::dialog::{FileDialogOptions, FileDialogType};
use crate::{FileInfo, FileSpec};

pub(crate) type NSModalResponse = NSInteger;
const NSModalResponseOK: NSInteger = 1;
const NSModalResponseCancel: NSInteger = 0;

pub(crate) unsafe fn get_file_info(
    panel: id,
    options: FileDialogOptions,
    result: NSModalResponse,
) -> Option<FileInfo> {
    match result {
        NSModalResponseOK => {
            let url: id = msg_send![panel, URL];
            let path: id = msg_send![url, path];
            let (path, format) = rewritten_path(panel, path, options);
            let path: OsString = from_nsstring(path).into();
            Some(FileInfo {
                path: path.into(),
                format,
            })
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
            // because other backends like Windows have no support for it,
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

    // If we have non-empty allowed types and a file save dialog,
    // we add a accessory view to set the file format
    match (&options.allowed_types, ty) {
        (Some(allowed_types), FileDialogType::Save) if !allowed_types.is_empty() => {
            let accessory_view = allowed_types_accessory_view(allowed_types);
            let _: () = msg_send![panel, setAccessoryView: accessory_view];
        }
        _ => (),
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
                tracing::warn!("The default type {:?} is not present in allowed types.", dt);
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

// AppKit has a built-in file format accessory view. However, this is only
// displayed for `NSDocument` based apps. We have to construct our own `NSView`
// hierarchy to implement something similar.
unsafe fn allowed_types_accessory_view(allowed_types: &[crate::FileSpec]) -> id {
    // Build the View Structure required to have file format popup.
    // This requires a container view, a label, a popup button,
    // and some layout code to make sure it behaves correctly for
    // fixed size and resizable views
    let total_frame = NSRect::new(
        NSPoint { x: 0.0, y: 0.0 },
        NSSize {
            width: 320.0,
            height: 30.0,
        },
    );

    let padding = 10.0;

    // Prepare the label
    let (label, label_size) = file_format_label();

    // Place the label centered (similar to the popup button)
    label.setFrameOrigin(NSPoint {
        x: padding,
        y: label_size.height / 2.0,
    });

    // Prepare the popup button
    let popup_frame = NSRect::new(
        NSPoint {
            x: padding + label_size.width + padding,
            y: 0.0,
        },
        NSSize {
            width: total_frame.size.width - (padding * 3.0) - label_size.width,
            height: total_frame.size.height,
        },
    );

    let popup_button = file_format_popup_button(allowed_types, popup_frame);

    // Prepare the container
    let container_view: id = msg_send![class!(NSView), alloc];
    let container_view: id = container_view.initWithFrame_(total_frame);

    container_view.addSubview_(label);
    container_view.addSubview_(popup_button);

    container_view.autorelease()
}

const FileFormatPopoverTag: NSInteger = 10;

unsafe fn file_format_popup_button(allowed_types: &[crate::FileSpec], popup_frame: NSRect) -> id {
    let popup_button: id = msg_send![class!(NSPopUpButton), alloc];
    let _: () = msg_send![popup_button, initWithFrame:popup_frame pullsDown:false];
    for allowed_type in allowed_types {
        let title = make_nsstring(allowed_type.name);
        msg_send![popup_button, addItemWithTitle: title]
    }
    let _: () = msg_send![popup_button, setTag: FileFormatPopoverTag];
    popup_button.autorelease()
}

unsafe fn file_format_label() -> (id, NSSize) {
    let label: id = msg_send![class!(NSTextField), new];
    let _: () = msg_send![label, setBezeled:false];
    let _: () = msg_send![label, setDrawsBackground:false];
    // FIXME: As we have to roll our own view hierarchy, we're not getting a translated
    // title here. So we ought to find a way to translate this.
    let title = make_nsstring("File Format:");
    let _: () = msg_send![label, setStringValue: title];
    let _: () = msg_send![label, sizeToFit];
    (label.autorelease(), label.frame().size)
}

/// Take a panel, a chosen path, and the file dialog options
/// and rewrite the path to utilize the chosen file format.
unsafe fn rewritten_path(
    panel: id,
    path: id,
    options: FileDialogOptions,
) -> (id, Option<FileSpec>) {
    let allowed_types = match options.allowed_types {
        Some(t) if !t.is_empty() => t,
        _ => return (path, None),
    };
    let accessory: id = msg_send![panel, accessoryView];
    if accessory == nil {
        return (path, None);
    }
    let popup_button: id = msg_send![accessory, viewWithTag: FileFormatPopoverTag];
    if popup_button == nil {
        return (path, None);
    }
    let index: NSInteger = msg_send![popup_button, indexOfSelectedItem];
    let file_spec = allowed_types[index as usize];
    let extension = file_spec.extensions[0];

    // Remove any extension the user might have entered and replace it with the
    // selected extension.
    // We're using `NSString` methods instead of `String` simply because
    // they are made for this purpose and simplify the implementation.
    let path: id = msg_send![path, stringByDeletingPathExtension];
    let path: id = msg_send![
        path,
        stringByAppendingPathExtension: make_nsstring(extension)
    ];
    (path, Some(file_spec))
}
