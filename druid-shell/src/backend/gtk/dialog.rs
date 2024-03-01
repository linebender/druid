// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! File open/save dialogs, GTK implementation.

use std::ffi::OsString;

use anyhow::anyhow;
use gtk::{FileChooserAction, FileFilter, ResponseType, Window};

use gtk::prelude::{FileChooserExt, NativeDialogExt};

use crate::dialog::{FileDialogOptions, FileDialogType, FileSpec};
use crate::Error;

fn file_filter(fs: &FileSpec) -> FileFilter {
    let ret = FileFilter::new();
    ret.set_name(Some(fs.name));
    for ext in fs.extensions {
        ret.add_pattern(&format!("*.{ext}"));
    }
    ret
}

pub(crate) fn get_file_dialog_path(
    window: &Window,
    ty: FileDialogType,
    options: FileDialogOptions,
) -> Result<Vec<OsString>, Error> {
    // TODO: support message localization

    let (title, action) = match (ty, options.select_directories) {
        (FileDialogType::Open, false) => ("Open File", FileChooserAction::Open),
        (FileDialogType::Open, true) => ("Open Folder", FileChooserAction::SelectFolder),
        (FileDialogType::Save, _) => ("Save File", FileChooserAction::Save),
    };
    let title = options.title.as_deref().unwrap_or(title);

    let mut dialog = gtk::FileChooserNative::builder()
        .transient_for(window)
        .title(title);
    if let Some(button_text) = &options.button_text {
        dialog = dialog.accept_label(button_text);
    }
    let dialog = dialog.build();

    dialog.set_action(action);

    dialog.set_show_hidden(options.show_hidden);

    if action != FileChooserAction::Save {
        dialog.set_select_multiple(options.multi_selection);
    }

    // Don't set the filters when showing the folder selection dialog,
    // because then folder traversing won't work.
    if action != FileChooserAction::SelectFolder {
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

        if let Some(dt) = &options.default_type {
            if !found_default_filter {
                tracing::warn!("The default type {:?} is not present in allowed types.", dt);
            }
        }
    }

    if let Some(default_name) = &options.default_name {
        dialog.set_current_name(default_name);
    }

    let result = dialog.run();

    let result = match result {
        ResponseType::Accept => match dialog.filenames() {
            filenames if filenames.is_empty() => Err(anyhow!("No path received for filename")),
            // If we receive more than one file, but `multi_selection` is false, return an error.
            filenames if filenames.len() > 1 && !options.multi_selection => {
                Err(anyhow!("More than one path received for single selection"))
            }
            // If we receive more than one file with a save action, return an error.
            filenames if filenames.len() > 1 && action == FileChooserAction::Save => {
                Err(anyhow!("More than one path received for save action"))
            }
            filenames => Ok(filenames.into_iter().map(|p| p.into_os_string()).collect()),
        },
        ResponseType::Cancel => Err(anyhow!("Dialog was deleted")),
        _ => {
            tracing::warn!("Unhandled dialog result: {:?}", result);
            Err(anyhow!("Unhandled dialog result"))
        }
    };

    // TODO properly handle errors into the Error type

    dialog.destroy();

    Ok(result?)
}
