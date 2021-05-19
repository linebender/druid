//! This module contains functions for opening file dialogs using DBus.
//!
//! TODO: The `ashpd` crate is going to provide a convenient high-level interface for this. Start
//! using that when it's ready.

use std::collections::HashMap;
use std::os::unix::ffi::OsStringExt;

use tracing::warn;
use zbus::{dbus_proxy, fdo, Connection};
use zvariant::{OwnedObjectPath, OwnedValue};

use super::window::IdleHandle;
use crate::{FileDialogOptions, FileDialogToken, FileInfo};

fn response_thread(
    conn: Connection,
    idle: IdleHandle,
    path: OwnedObjectPath,
    tok: FileDialogToken,
    open: bool,
) -> fdo::Result<()> {
    let path = path.into_inner();
    loop {
        let msg = conn.receive_message()?;
        let msg_header = msg.header()?;
        if msg_header.message_type()? == zbus::MessageType::Signal
            && msg_header.member()? == Some("Response")
        {
            if let Some(reply_path) = msg.header()?.path()? {
                if &path == reply_path {
                    let response = msg.body::<ResponseResult<FileDialogResponse>>()?;
                    let file_info = if let ResponseType::Success = response.0 {
                        // FIXME: what are we supposed to do if there are multiple files?
                        response
                            .1
                            .uris
                            .first()
                            .and_then(|s| s.strip_prefix("file://"))
                            .map(|s| FileInfo { path: s.into() })
                    } else {
                        None
                    };
                    idle.add_idle_callback(move |handler| {
                        if open {
                            handler.open_file(tok, file_info);
                        } else {
                            handler.save_as(tok, file_info);
                        }
                    });
                    return Ok(());
                } else {
                    warn!("got path {:?}, expected {:?}", reply_path, path);
                }
            }
        }
    }
}

pub(crate) fn open_file(
    window: u32,
    idle: IdleHandle,
    options: FileDialogOptions,
) -> fdo::Result<FileDialogToken> {
    let connection = Connection::new_session()?;
    let proxy = FileChooserProxy::new(&connection)?;

    let title = options
        .title
        .clone()
        .unwrap_or_else(|| "Open file".to_owned());
    let opts = OpenOptions::from(options);
    let reply = proxy.open_file(&format!("x11:{}", window), &title, opts)?;
    let tok = FileDialogToken::next();

    std::thread::spawn(move || response_thread(connection, idle, reply, tok, true));

    Ok(tok)
}

pub(crate) fn save_file(
    window: u32,
    idle: IdleHandle,
    options: FileDialogOptions,
) -> fdo::Result<FileDialogToken> {
    let connection = Connection::new_session()?;
    let proxy = FileChooserProxy::new(&connection)?;

    let title = options
        .title
        .clone()
        .unwrap_or_else(|| "Save as".to_owned());
    let opts = SaveOptions::from(options);
    let reply = proxy.save_file(&format!("x11:{}", window), &title, opts)?;
    let tok = FileDialogToken::next();

    std::thread::spawn(move || response_thread(connection, idle, reply, tok, false));

    Ok(tok)
}

/// The options supported by the freedesktop.org filechooser protocol. These are documented at
/// https://flatpak.github.io/xdg-desktop-portal/portal-docs.html#gdbus-org.freedesktop.portal.FileChooser
#[derive(zvariant_derive::SerializeDict, zvariant_derive::TypeDict)]
// the dbus_proxy macro makes me make this pub, but it isn't actually really pub because this
// module is private
pub struct OpenOptions {
    /// Label for the accept button. FDO says that mnemonic underlines are supported; does the
    /// druid end use mnemonic underlines also? (TODO)
    accept_label: Option<String>,
    /// Whether the dialog should be modal. Ours are always modal.
    modal: bool,
    /// Do we allow selecting multiple files?
    multiple: bool,
    /// Whether to select folders instead of files.
    directory: bool,
    /// The list of allowed file filters.
    filters: Vec<Filter>,
    /// The default filter.
    current_filter: Option<Filter>,
}

/// The options supported by the freedesktop.org filechooser protocol. These are documented at
/// https://flatpak.github.io/xdg-desktop-portal/portal-docs.html#gdbus-org.freedesktop.portal.FileChooser
#[derive(zvariant_derive::SerializeDict, zvariant_derive::TypeDict)]
pub struct SaveOptions {
    /// Label for the accept button. FDO says that mnemonic underlines are supported; does the
    /// druid end use mnemonic underlines also? (TODO)
    accept_label: Option<String>,
    /// Whether the dialog should be modal. Ours are always modal.
    modal: bool,
    /// The list of allowed file filters.
    filters: Vec<Filter>,
    /// The default filter.
    current_filter: Option<Filter>,
    /// The suggested filename.
    current_name: Option<String>,
    /// The starting directory.
    current_folder: Option<Vec<u8>>,
}

/// A file filter.
///
/// The first argument is a description of the filter (e.g. "Images"). The second argument is a
/// list of allowed file types. The `u32` determines the meaning of the string: FDO supports both
/// mimetypes and glob patterns (but we only use glob patterns).
#[derive(serde::Serialize, zvariant_derive::Type)]
struct Filter(&'static str, Vec<(u32, String)>);

impl From<crate::FileDialogOptions> for OpenOptions {
    fn from(opts: crate::FileDialogOptions) -> OpenOptions {
        OpenOptions {
            accept_label: opts.button_text,
            modal: true,
            multiple: opts.multi_selection,
            directory: opts.select_directories,
            filters: opts
                .allowed_types
                .unwrap_or_else(Vec::new)
                .into_iter()
                .map(Into::into)
                .collect(),
            current_filter: opts.default_type.map(Into::into),
        }
    }
}

impl From<crate::FileDialogOptions> for SaveOptions {
    fn from(opts: crate::FileDialogOptions) -> SaveOptions {
        SaveOptions {
            accept_label: opts.button_text,
            modal: true,
            filters: opts
                .allowed_types
                .unwrap_or_else(Vec::new)
                .into_iter()
                .map(Into::into)
                .collect(),
            current_filter: opts.default_type.map(Into::into),
            current_name: opts.default_name,
            current_folder: opts
                .starting_directory
                .map(|p| p.into_os_string().into_vec()),
        }
    }
}

impl From<crate::FileSpec> for Filter {
    fn from(fs: crate::FileSpec) -> Filter {
        Filter(
            fs.name,
            fs.extensions
                .iter()
                .map(|ext| (0, format!("*.{}", ext)))
                .collect(),
        )
    }
}

/// The response from a DBus method call contains an error code, followed by the actual data we
/// asked for.
#[derive(serde::Deserialize, Debug)]
struct ResponseResult<T>(ResponseType, T);

/// The response to a file dialog request.
#[derive(zvariant_derive::DeserializeDict, Debug)]
struct FileDialogResponse {
    /// The list of selected files.
    uris: Vec<String>,
    /// The DBus protocol allows for the file dialog to contain various combo-boxes. We don't use
    /// them.
    choices: Option<Vec<(String, String)>>,
}

impl zvariant::Type for FileDialogResponse {
    fn signature() -> zvariant::Signature<'static> {
        <HashMap<&str, OwnedValue>>::signature()
    }
}

impl<T: zvariant::Type> zvariant::Type for ResponseResult<T> {
    fn signature() -> zvariant::Signature<'static> {
        <(u32, T)>::signature()
    }
}

#[dbus_proxy(
    interface = "org.freedesktop.portal.FileChooser",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait FileChooser {
    fn open_file(
        &self,
        parent_window: &str,
        title: &str,
        options: OpenOptions,
    ) -> fdo::Result<OwnedObjectPath>;

    fn save_file(
        &self,
        parent_window: &str,
        title: &str,
        options: SaveOptions,
    ) -> fdo::Result<OwnedObjectPath>;
}

#[derive(Clone, Copy, Debug, serde::Deserialize, zvariant_derive::Type)]
#[repr(u32)]
enum ResponseType {
    Success = 0,
    Cancelled = 1,
    Other = 2,
}
