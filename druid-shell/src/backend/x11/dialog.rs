// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This module contains functions for opening file dialogs using DBus.

use ashpd::desktop::file_chooser;
use ashpd::{zbus, WindowIdentifier};
use futures::executor::block_on;
use tracing::warn;

use crate::{FileDialogOptions, FileDialogToken, FileInfo};

use super::window::IdleHandle;

pub(crate) fn open_file(
    window: u32,
    idle: IdleHandle,
    options: FileDialogOptions,
) -> FileDialogToken {
    dialog(window, idle, options, true)
}

pub(crate) fn save_file(
    window: u32,
    idle: IdleHandle,
    options: FileDialogOptions,
) -> FileDialogToken {
    dialog(window, idle, options, false)
}

fn dialog(
    window: u32,
    idle: IdleHandle,
    mut options: FileDialogOptions,
    open: bool,
) -> FileDialogToken {
    let tok = FileDialogToken::next();

    std::thread::spawn(move || {
        if let Err(e) = block_on(async {
            let conn = zbus::Connection::session().await?;
            let proxy = file_chooser::FileChooserProxy::new(&conn).await?;
            let id = WindowIdentifier::from_xid(window as u64);
            let multi = options.multi_selection;

            let title_owned = options.title.take();
            let title = match (open, options.select_directories) {
                (true, true) => "Open Folder",
                (true, false) => "Open File",
                (false, _) => "Save File",
            };
            let title = title_owned.as_deref().unwrap_or(title);
            let open_result;
            let save_result;
            let uris = if open {
                open_result = proxy.open_file(&id, title, options.into()).await?;
                open_result.uris()
            } else {
                save_result = proxy.save_file(&id, title, options.into()).await?;
                save_result.uris()
            };

            let mut paths = uris.iter().filter_map(|s| {
                s.strip_prefix("file://").or_else(|| {
                    warn!("expected path '{}' to start with 'file://'", s);
                    None
                })
            });
            if multi && open {
                let infos = paths
                    .map(|p| FileInfo {
                        path: p.into(),
                        format: None,
                    })
                    .collect();
                idle.add_idle_callback(move |handler| handler.open_files(tok, infos));
            } else if !multi {
                if uris.len() > 2 {
                    warn!(
                        "expected one path (got {}), returning only the first",
                        uris.len()
                    );
                }
                let info = paths.next().map(|p| FileInfo {
                    path: p.into(),
                    format: None,
                });
                if open {
                    idle.add_idle_callback(move |handler| handler.open_file(tok, info));
                } else {
                    idle.add_idle_callback(move |handler| handler.save_as(tok, info));
                }
            } else {
                warn!("cannot save multiple paths");
            }

            Ok(()) as ashpd::Result<()>
        }) {
            warn!("error while opening file dialog: {}", e);
        }
    });

    tok
}

impl From<crate::FileSpec> for file_chooser::FileFilter {
    fn from(spec: crate::FileSpec) -> file_chooser::FileFilter {
        let mut filter = file_chooser::FileFilter::new(spec.name);
        for ext in spec.extensions {
            filter = filter.glob(&format!("*.{ext}"));
        }
        filter
    }
}

impl From<crate::FileDialogOptions> for file_chooser::OpenFileOptions {
    fn from(opts: crate::FileDialogOptions) -> file_chooser::OpenFileOptions {
        let mut fc = file_chooser::OpenFileOptions::default()
            .modal(true)
            .multiple(opts.multi_selection)
            .directory(opts.select_directories);

        if let Some(label) = &opts.button_text {
            fc = fc.accept_label(label);
        }

        if let Some(filters) = opts.allowed_types {
            for f in filters {
                fc = fc.add_filter(f.into());
            }
        }

        if let Some(filter) = opts.default_type {
            fc = fc.current_filter(filter.into());
        }

        fc
    }
}

impl From<crate::FileDialogOptions> for file_chooser::SaveFileOptions {
    fn from(opts: crate::FileDialogOptions) -> file_chooser::SaveFileOptions {
        let mut fc = file_chooser::SaveFileOptions::default().modal(true);

        if let Some(name) = &opts.default_name {
            fc = fc.current_name(name);
        }

        if let Some(label) = &opts.button_text {
            fc = fc.accept_label(label);
        }

        if let Some(filters) = opts.allowed_types {
            for f in filters {
                fc = fc.add_filter(f.into());
            }
        }

        if let Some(filter) = opts.default_type {
            fc = fc.current_filter(filter.into());
        }

        if let Some(dir) = &opts.starting_directory {
            fc = fc.current_folder(dir);
        }

        fc
    }
}
