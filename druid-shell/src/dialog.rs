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
#[cfg(not(all(target_os = "linux", feature = "x11")))]
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
/// will be a directory when the user chooses a package. You can read more about [packages] below.
/// It's also possible for users to manually specify a path which they might otherwise not be able
/// to choose. Thus it is important to verify that all the returned paths match your expectations.
///
/// The open dialog can also be switched to *directories mode* via [`select_directories`].
///
/// # Cross-platform compatibility
///
/// You could write platform specific code that really makes the best use of each platform.
/// However if you want to write universal code that will work on all platforms then
/// you have to keep some restrictions in mind.
///
/// ## Don't depend on directories with extensions
///
/// Your application should avoid having to deal with directories that have extensions
/// in their name, e.g. `my_stuff.pkg`. This clashes with [packages] on macOS and you
/// will either need platform specific code or a degraded user experience on macOS
/// via [`packages_as_directories`].
///
/// ## Use the save dialog only for new paths
///
/// Don't direct the user to choose an existing file with the save dialog.
/// Selecting existing files for overwriting is possible but extremely cumbersome on macOS.
/// The much more optimized flow is to have the user select a file with the open dialog
/// and then keep saving to that file without showing a save dialog.
/// Use the save dialog only for selecting a new location.
///
/// # macOS
///
/// The file dialog works a bit differently on macOS. For a lot of applications this doesn't matter
/// and you don't need to know the details. However if your application makes extensive use
/// of file dialogs and you target macOS then you should understand the macOS specifics.
///
/// ## Packages
///
/// On macOS directories with known extensions are considered to be packages, e.g. `app_files.pkg`.
/// Furthermore the packages are divided into two groups based on their extension.
/// First there are packages that have been defined at the OS level, and secondly there are
/// packages that are defined at the file dialog level based on [`allowed_types`].
/// These two types have slightly different behavior in the file dialogs. Generally packages
/// behave similarly to regular files in many contexts, including the file dialogs.
/// This package concept can be turned off in the file dialog via [`packages_as_directories`].
///
/// &#xFEFF; | Packages as files. File filters apply to packages. | Packages as directories.
/// -------- | -------------------------------------------------- | ------------------------
/// Open directory | Not selectable. Not traversable. | Selectable. Traversable.
/// Open file | Selectable. Not traversable. | Not selectable. Traversable.
/// Save file | OS packages [clickable] but not traversable.<br/>Dialog packages traversable but not selectable. | Not selectable. Traversable.
///
/// Keep in mind that the file dialog may start inside any package if the user has traversed
/// into one just recently. The user might also manually specify a path inside a package.
///
/// Generally this behavior should be kept, because it's least surprising to macOS users.
/// However if your application requires selecting directories with extensions as directories
/// or the user needs to be able to traverse into them to select a specific file,
/// then you can change the default behavior via [`packages_as_directories`]
/// to force macOS to behave like other platforms and not give special treatment to packages.
///
/// ## Selecting files for overwriting in the save dialog is cumbersome
///
/// Existing files can be clicked on in the save dialog, but that only copies their base file name.
/// If the clicked file's extension is different than the first extension of the default type
/// then the returned path does not actually match the path of the file that was clicked on.
/// Clicking on a file doesn't change the base path either. Keep in mind that the macOS file dialog
/// can have several directories open at once. So if a user has traversed into `/Users/Joe/foo/`
/// and then clicks on an existing file `/Users/Joe/old.txt` in another directory then the returned
/// path will actually be `/Users/Joe/foo/old.rtf` if the default type's first extension is `rtf`.
///
/// ## Have a really good save dialog default type
///
/// There is no way for the user to choose which extension they want to save a file as via the UI.
/// They have no way of knowing which extensions are even supported and must manually type it out.
///
/// *Hopefully it's a temporary problem and we can find a way to show the file formats in the UI.
/// This is being tracked in [druid#998].*
///
/// [clickable]: #selecting-files-for-overwriting-in-the-save-dialog-is-cumbersome
/// [packages]: #packages
/// [`select_directories`]: #method.select_directories
/// [`allowed_types`]: #method.allowed_types
/// [`packages_as_directories`]: #method.packages_as_directories
/// [druid#998]: https://github.com/xi-editor/druid/issues/998
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
    /// This allows for writing more universal cross-platform code at the cost of user experience.
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
