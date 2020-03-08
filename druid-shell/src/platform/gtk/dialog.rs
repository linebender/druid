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

use crate::dialog::{FileDialogOptions, FileDialogType};
use gtk::{FileChooserAction, FileChooserExt, NativeDialogExt, ResponseType, Window};

use crate::Error;

pub(crate) fn get_file_dialog_path(
    window: &Window,
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Result<OsString, Error> {
    // TODO: support message localization

    let (title, action) = match (ty, options.select_directories) {
        (FileDialogType::Open, false) => ("Open File", FileChooserAction::Open),
        (FileDialogType::Open, true) => ("Open File", FileChooserAction::SelectFolder),
        (FileDialogType::Save, _) => ("Save File", FileChooserAction::Save),
    };

    let dialog = gtk::FileChooserNativeBuilder::new()
        .transient_for(window)
        .title(title)
        .build();

    dialog.set_action(action);

    dialog.set_show_hidden(options.show_hidden);

    dialog.set_select_multiple(options.multi_selection);

    // dialog.set_show_hidden(options.select_directories);

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
