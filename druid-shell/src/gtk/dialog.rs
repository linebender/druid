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

use gtkrs::{
    DialogExt, FileChooserAction, FileChooserDialog, FileChooserExt, ResponseType, WidgetExt,
    Window,
};

use crate::Error;

/// Type of file dialog.
pub enum FileDialogType {
    /// File open dialog.
    Open,
    /// File save dialog.
    Save,
}

/// Options for file dialog.
#[derive(Default)]
pub struct FileDialogOptions {
    show_hidden: bool,
}

impl FileDialogOptions {
    pub fn set_show_hidden(&mut self) {
        self.show_hidden = true
    }
}

pub(crate) fn get_file_dialog_path(
    window: &Window,
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Result<OsString, Error> {
    let dialog = match ty {
        FileDialogType::Open => build_open_dialog(window, options),
        FileDialogType::Save => build_save_dialog(window, options),
    };

    let result = dialog.run();

    let result = match result {
        ResponseType::Accept => match dialog.get_filename() {
            Some(path) => Ok(path.into_os_string()),
            None => Err(Error::Other("No path received for filename")),
        },
        ResponseType::DeleteEvent => Err(Error::Other("Dialog was deleted")),
        _ => {
            eprintln!("Unhandled dialog result: {:?}", result);
            Err(Error::Other("Unhandled dialog result"))
        }
    };

    // TODO properly handle errors into the Error type

    dialog.destroy();

    result
}

// TODO DRY this up
fn build_open_dialog(window: &Window, options: FileDialogOptions) -> FileChooserDialog {
    let dialog = gtk::FileChooserDialogBuilder::new()
        .transient_for(window)
        .title("Open File")
        .build();

    dialog.set_action(FileChooserAction::Open);

    dialog.add_button("Open", ResponseType::Accept);
    dialog.add_button("Cancel", ResponseType::Cancel);

    dialog.set_show_hidden(options.show_hidden);

    dialog
}

fn build_save_dialog(window: &Window, options: FileDialogOptions) -> FileChooserDialog {
    let dialog = gtk::FileChooserDialogBuilder::new()
        .transient_for(window)
        .title("Save File")
        .build();

    dialog.set_action(FileChooserAction::Save);

    dialog.add_button("Save", ResponseType::Accept);
    dialog.add_button("Cancel", ResponseType::Cancel);

    dialog.set_show_hidden(options.show_hidden);

    dialog
}
