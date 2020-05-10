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

//! File open/save dialogs, GTK implementation.

use std::ffi::OsString;

use gtk::{FileChooserAction, FileChooserExt, FileFilter, NativeDialogExt, ResponseType, Window};

use crate::dialog::{FileDialogOptions, FileDialogType, FileSpec};
use crate::Error;

fn file_filter(fs: &FileSpec) -> FileFilter {
    let ret = FileFilter::new();
    ret.set_name(Some(fs.name));
    for ext in fs.extensions {
        ret.add_pattern(&format!("*.{}", ext));
    }
    ret
}

pub(crate) fn get_file_dialog_path(
    window: &Window,
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Result<OsString, Error> {
    // TODO: support message localization

    let (title, action) = match (ty, options.select_directories) {
        (FileDialogType::Open, false) => ("Open File", FileChooserAction::Open),
        (FileDialogType::Open, true) => ("Open Folder", FileChooserAction::SelectFolder),
        (FileDialogType::Save, _) => ("Save File", FileChooserAction::Save),
    };

    let dialog = gtk::FileChooserNativeBuilder::new()
        .transient_for(window)
        .title(title)
        .build();

    dialog.set_action(action);

    dialog.set_show_hidden(options.show_hidden);

    dialog.set_select_multiple(options.multi_selection);

    let mut found_default_filter = false;
    if let Some(file_types) = &options.allowed_types {
        for f in file_types {
            let filter = file_filter(f);
            dialog.add_filter(&filter);

            if let Some(default) = &options.default_type {
                if default == f {
                    // Note that we're providing the same FileFilter object to
                    // add_filter and set_filter, because gtk checks them for
                    // identity, not structural equality.
                    dialog.set_filter(&filter);
                    found_default_filter = true;
                }
            }
        }
    }

    if let Some(default_file_type) = &options.default_type {
        if options.allowed_types.is_some() && !found_default_filter {
            // It's ok to set a default file filter without providing a list of
            // allowed filters, but it's not ok (or at least, doesn't work in gtk)
            // to provide a default filter that isn't in the (present) list
            // of allowed filters.
            log::warn!("default file type not found in allowed types");
        } else if !found_default_filter {
            dialog.set_filter(&file_filter(default_file_type));
        }
    }

    let result = dialog.run();

    let result = match result {
        ResponseType::Accept => match dialog.get_filename() {
            Some(path) => Ok(path.into_os_string()),
            None => Err(Error::Other("No path received for filename")),
        },
        ResponseType::Cancel => Err(Error::Other("Dialog was deleted")),
        _ => {
            log::warn!("Unhandled dialog result: {:?}", result);
            Err(Error::Other("Unhandled dialog result"))
        }
    };

    // TODO properly handle errors into the Error type

    dialog.destroy();

    result
}
