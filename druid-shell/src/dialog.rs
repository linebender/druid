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

//! File open/save dialogs.

use std::path::{Path, PathBuf};

/// Information about the path to be opened or saved.
///
/// This path might point to a file or a directory.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub(crate) path: PathBuf,
}

/// Type of file dialog.
#[cfg(not(feature = "x11"))]
pub enum FileDialogType {
    /// File open dialog.
    Open,
    /// File save dialog.
    Save,
}

/// Options for file dialogs.
///
/// File dialogs let the user choose a specific path to open or save.
///
/// By default the file dialogs operate in *files mode* where the user can only choose files.
/// Importantly these are files from the user's perspective, but technically the returned path
/// will be a directory when the user chooses a package. Read more about [packages] below.
/// Thus it is important to verify that all the returned paths match your expectations.
///
/// The open dialog can also be switched to *directories mode* via [`select_directories`].
///
/// # Packages
///
/// On macOS directories with known extensions are considered to be packages.
/// Furthermore the packages are divided into two groups based on their extension.
/// First there are packages that have been defined at the OS level, and secondly there are
/// packages that are defined at the file dialog level based on [`allowed_types`].
/// A package behaves similarly to a regular file in many contexts, including the file dialogs.
/// This means that it can be chosen in the file open dialog when operating in *files mode*
/// and all the file filters will apply to it. It won't be selectable in *directories mode*
/// and the user won't be able to traverse into the package. In the file save dialog
/// packages that are only defined in [`allowed_types`] can be traversed into. However
/// packages that are defined at the OS level can't be traversed into even if they are also
/// defined in [`allowed_types`]. Keep in mind though that the file dialog may start
/// inside any package if the user had traversed into one previously.
///
/// Generally this behavior should be kept, because it's least surprising to macOS users.
/// However if your application requires selecting directories with extensions as directories
/// or the user needs to be able to traverse into them to select a specific file,
/// then you can change the default behavior via [`packages_as_directories`]
/// to force macOS to behave like other platforms and not give special treatment to packages.
///
/// [packages]: #packages
/// [`select_directories`]: #method.select_directories
/// [`allowed_types`]: #method.allowed_types
/// [`packages_as_directories`]: #method.packages_as_directories
#[derive(Debug, Clone, Default)]
pub struct FileDialogOptions {
    pub(crate) show_hidden: bool,
    pub(crate) allowed_types: Option<Vec<FileSpec>>,
    pub(crate) default_type: Option<FileSpec>,
    pub(crate) select_directories: bool,
    pub(crate) packages_as_directories: bool,
    pub(crate) multi_selection: bool,
    pub(crate) default_name: Option<String>,
    pub(crate) name_label: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) button_text: Option<String>,
    pub(crate) starting_directory: Option<PathBuf>,
}

/// A description of a filetype, for specifiying allowed types in a file dialog.
///
/// # Windows
///
/// Each instance of this type is converted to a [`COMDLG_FILTERSPEC`] struct.
///
/// # macOS
///
/// These file types also apply to directories to define them as [packages].
///
/// [`COMDLG_FILTERSPEC`]: https://docs.microsoft.com/en-ca/windows/win32/api/shtypes/ns-shtypes-comdlg_filterspec
/// [packages]: struct.FileDialogOptions.html#packages
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileSpec {
    /// A human readable name, describing this filetype.
    ///
    /// This is used in the Windows file dialog, where the user can select
    /// from a dropdown the type of file they would like to choose.
    ///
    /// This should not include the file extensions; they will be added automatically.
    /// For instance, if we are describing Word documents, the name would be "Word Document",
    /// and the displayed string would be "Word Document (*.doc)".
    pub name: &'static str,
    /// The file extensions used by this file type.
    ///
    /// This should not include the leading '.'.
    pub extensions: &'static [&'static str],
}

impl FileInfo {
    /// Returns the underlying path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl FileDialogOptions {
    /// Create a new set of options.
    pub fn new() -> FileDialogOptions {
        FileDialogOptions::default()
    }

    /// Set hidden files and directories to be visible.
    pub fn show_hidden(mut self) -> Self {
        self.show_hidden = true;
        self
    }

    /// Set directories to be selectable instead of files.
    ///
    /// This is only relevant for open dialogs.
    pub fn select_directories(mut self) -> Self {
        self.select_directories = true;
        self
    }

    /// Set [packages] to be treated as directories instead of files.
    ///
    /// This is only relevant on macOS.
    ///
    /// [packages]: #packages
    pub fn packages_as_directories(mut self) -> Self {
        self.packages_as_directories = true;
        self
    }

    /// Set multiple items to be selectable.
    ///
    /// This is only relevant for open dialogs.
    pub fn multi_selection(mut self) -> Self {
        self.multi_selection = true;
        self
    }

    /// Set the file types the user is allowed to select.
    ///
    /// This filter is only applied to files and [packages], but not to directories.
    ///
    /// An empty collection is treated as no filter.
    ///
    /// # macOS
    ///
    /// These file types also apply to directories to define [packages].
    /// Which means the directories that match the filter are no longer considered directories.
    /// The packages are defined by this collection even in *directories mode*.
    ///
    /// [packages]: #packages
    pub fn allowed_types(mut self, types: Vec<FileSpec>) -> Self {
        // An empty vector can cause platform issues, so treat it as no filter
        if types.is_empty() {
            self.allowed_types = None;
        } else {
            self.allowed_types = Some(types);
        }
        self
    }

    /// Set the default file type.
    ///
    /// The provided `default_type` must also be present in [`allowed_types`].
    ///
    /// If it's `None` then the first entry in [`allowed_types`] will be used as the default.
    ///
    /// This is only relevant in *files mode*.
    ///
    /// [`allowed_types`]: #method.allowed_types
    pub fn default_type(mut self, default_type: FileSpec) -> Self {
        self.default_type = Some(default_type);
        self
    }

    /// Set the default filename that appears in the dialog.
    pub fn default_name(mut self, default_name: impl Into<String>) -> Self {
        self.default_name = Some(default_name.into());
        self
    }

    /// Set the text in the label next to the filename editbox.
    pub fn name_label(mut self, name_label: impl Into<String>) -> Self {
        self.name_label = Some(name_label.into());
        self
    }

    /// Set the title text of the dialog.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the text of the Open/Save button.
    pub fn button_text(mut self, text: impl Into<String>) -> Self {
        self.button_text = Some(text.into());
        self
    }

    /// Force the starting directory to the specified `path`.
    ///
    /// # User experience
    ///
    /// This should almost never be used because it overrides the OS choice,
    /// which will usually be a directory that the user recently visited.
    pub fn force_starting_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.starting_directory = Some(path.into());
        self
    }
}

impl FileSpec {
    pub const TEXT: FileSpec = FileSpec::new("Text", &["txt"]);
    pub const JPG: FileSpec = FileSpec::new("Jpeg", &["jpg", "jpeg"]);
    pub const GIF: FileSpec = FileSpec::new("Gif", &["gif"]);
    pub const PDF: FileSpec = FileSpec::new("PDF", &["pdf"]);
    pub const HTML: FileSpec = FileSpec::new("Web Page", &["htm", "html"]);

    /// Create a new `FileSpec`.
    pub const fn new(name: &'static str, extensions: &'static [&'static str]) -> Self {
        FileSpec { name, extensions }
    }
}
