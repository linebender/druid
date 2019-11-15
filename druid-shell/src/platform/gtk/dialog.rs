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

use crate::dialog::FileDialogOptions;
use gtk::{FileChooserAction, FileChooserExt, NativeDialogExt, Window};

use crate::Error;

/// Type of file dialog.
pub enum FileDialogType {
    /// File open dialog.
    Open,
    /// File save dialog.
    Save,
}

pub(crate) fn open_file_sync(
    window: &Window,
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Result<OsString, Error> {
    // TODO: support message localization
    let (title, action) = match ty {
        FileDialogType::Open => ("Open File", FileChooserAction::Open),
        FileDialogType::Save => ("Save File", FileChooserAction::Save),
    };

    let dialog = gtk::FileChooserNativeBuilder::new()
        .transient_for(window)
        .title(title)
        .build();

    dialog.set_action(action);

    dialog.set_show_hidden(options.show_hidden);

    let result = dialog.run();

    let result = match result {
        gtk_sys::GTK_RESPONSE_ACCEPT => match dialog.get_filename() {
            Some(path) => Ok(path.into_os_string()),
            None => Err(Error::Other("No path received for filename")),
        },
        gtk_sys::GTK_RESPONSE_CANCEL => Err(Error::Other("Dialog was deleted")),
        _ => {
            eprintln!("Unhandled dialog result: {:?}", result);
            Err(Error::Other("Unhandled dialog result"))
        }
    };

    // TODO properly handle errors into the Error type

    dialog.destroy();

    result
}
