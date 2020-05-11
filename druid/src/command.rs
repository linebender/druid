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

//! Custom commands.

use std::any::Any;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{WidgetId, WindowId};

/// An untyped identifier for a `Selector`.
pub type SelectorSymbol = &'static str;

/// An identifier for a particular command.
///
/// This should be a unique string identifier. Certain `Selector`s are defined
/// by druid, and have special meaning to the framework; these are listed in the
/// [`druid::commands`] module.
///
/// [`druid::commands`]: commands/index.html
#[derive(Debug, PartialEq, Eq)]
pub struct Selector<T>(SelectorSymbol, PhantomData<*const T>);

// This has do be done explicitly, to avoid the Copy bound on `T`.
// See https://doc.rust-lang.org/std/marker/trait.Copy.html#how-can-i-implement-copy .
impl<T> Copy for Selector<T> {}
impl<T> Clone for Selector<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// An identifier for a particular command.
///
/// This should be a unique string identifier. Certain `Selector`s are defined
/// by druid, and have special meaning to the framework; these are listed in the
/// [`druid::commands`] module.
///
/// [`druid::commands`]: commands/index.html
#[derive(Debug, PartialEq, Eq)]
pub struct OneShotSelector<T>(SelectorSymbol, PhantomData<*const T>);

// This has do be done explicitly, to avoid the Copy bound on `T`.
// See https://doc.rust-lang.org/std/marker/trait.Copy.html#how-can-i-implement-copy .
impl<T> Copy for OneShotSelector<T> {}
impl<T> Clone for OneShotSelector<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub trait AnySelector {
    fn symbol(self) -> SelectorSymbol;
}

impl<T> AnySelector for Selector<T> {
    fn symbol(self) -> SelectorSymbol {
        self.0
    }
}

impl<T> AnySelector for OneShotSelector<T> {
    fn symbol(self) -> SelectorSymbol {
        self.0
    }
}

/// An arbitrary command.
///
/// A `Command` consists of a [`Selector`], that indicates what the command is,
/// and an optional argument, that can be used to pass arbitrary data.
///
///
/// # One-shot and reusable `Commands`
///
/// Commands come in two varieties, 'reusable' and 'one-shot'.
///
/// Regular commands are created with [`Command::new`], and their argument
/// objects may be accessed repeatedly, via [`Command::get_object`].
///
/// One-shot commands are intended for cases where an object should only be
/// used once; an example would be if you have some resource that cannot be
/// cloned, and you wish to send it to another widget.
///
/// # Examples
/// ```
/// use druid::{Command, Selector};
///
/// let selector = Selector::new("process_rows");
/// let rows = vec![1, 3, 10, 12];
/// let command = Command::new(selector, rows);
///
/// assert_eq!(command.get(selector), Some(&vec![1, 3, 10, 12]));
/// ```
///
/// [`Command::new`]: #method.new
/// [`Command::get_object`]: #method.get_object
/// [`Selector`]: struct.Selector.html
#[derive(Debug, Clone)]
pub struct Command {
    selector: SelectorSymbol,
    object: Arg,
}

#[derive(Debug, Clone)]
enum Arg {
    Reusable(Arc<dyn Any>),
    OneShot(Arc<Mutex<Option<Box<dyn Any>>>>),
}

/// Errors that can occur when attempting to retrieve the a `OneShotCommand`s argument.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentError {
    /// The command represented a different selector.
    WrongSelector,
    /// The one-shot argument has already been taken.
    Consumed,
}

/// This error can occure when wrongly promising that a type ereased
/// variant of some generic item represents the application state.
/// Examples are `MenuDesc<T>` and `AppStateMenuDesc`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppStateTypeError {
    expected: &'static str,
    found: &'static str,
}

impl AppStateTypeError {
    pub(crate) fn new(expected: &'static str, found: &'static str) -> Self {
        Self { expected, found }
    }
}

impl std::fmt::Display for AppStateTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Promise to represent the app state was not meet. Expected {} but got {}.",
            self.expected, self.found
        )
    }
}

impl std::error::Error for AppStateTypeError {}

/// The target of a command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// The target is the top-level application.
    Global,
    /// The target is a window; the event will be delivered to all
    /// widgets in that window.
    Window(WindowId),
    /// The target is a specific widget.
    Widget(WidgetId),
}

/// Commands with special meaning, defined by druid.
///
/// See [`Command`] for more info.
///
/// [`Command`]: ../struct.Command.html
pub mod sys {
    use super::{OneShotSelector, Selector};
    use crate::{
        app::AppStateWindowDesc,
        menu::{AppStateContextMenu, AppStateMenuDesc},
        FileDialogOptions, FileInfo,
    };

    /// Quit the running application. This command is handled by the druid library.
    pub const QUIT_APP: Selector<()> = Selector::new("druid-builtin.quit-app");

    /// Hide the application. (mac only?)
    pub const HIDE_APPLICATION: Selector<()> = Selector::new("druid-builtin.menu-hide-application");

    /// Hide all other applications. (mac only?)
    pub const HIDE_OTHERS: Selector<()> = Selector::new("druid-builtin.menu-hide-others");

    /// The selector for a command to create a new window.
    pub const NEW_WINDOW: OneShotSelector<AppStateWindowDesc> =
        OneShotSelector::new("druid-builtin.new-window");

    /// The selector for a command to close a window.
    ///
    /// The command must target a specific window.
    /// When calling `submit_command` on a `Widget`s context, passing `None` as target
    /// will automatically target the window containing the widget.
    pub const CLOSE_WINDOW: Selector<()> = Selector::new("druid-builtin.close-window");

    /// Close all windows.
    pub const CLOSE_ALL_WINDOWS: Selector<()> = Selector::new("druid-builtin.close-all-windows");

    /// The selector for a command to bring a window to the front, and give it focus.
    ///
    /// The command must target a specific window.
    /// When calling `submit_command` on a `Widget`s context, passing `None` as target
    /// will automatically target the window containing the widget.
    pub const SHOW_WINDOW: Selector<()> = Selector::new("druid-builtin.show-window");

    /// Display a context (right-click) menu.
    /// An `AppStateContextMenu` can be obtained using `ContextMenu::into_app_state_context_menu`.
    ///
    /// [`ContextMenu`]: ../struct.ContextMenu.html
    pub const SHOW_CONTEXT_MENU: Selector<AppStateContextMenu> =
        Selector::new("druid-builtin.show-context-menu");

    /// The selector for a command to set the window's menu.
    /// An `AppStateMenuDesc` can be obtained using `MenuDesc::into_app_state_menu_desc`.
    ///
    /// [`MenuDesc`]: ../struct.MenuDesc.html
    pub const SET_MENU: Selector<AppStateMenuDesc> = Selector::new("druid-builtin.set-menu");

    /// Show the application preferences.
    pub const SHOW_PREFERENCES: Selector<()> = Selector::new("druid-builtin.menu-show-preferences");

    /// Show the application about window.
    pub const SHOW_ABOUT: Selector<()> = Selector::new("druid-builtin.menu-show-about");

    /// Show all applications.
    pub const SHOW_ALL: Selector<()> = Selector::new("druid-builtin.menu-show-all");

    /// Show the new file dialog.
    pub const NEW_FILE: Selector<()> = Selector::new("druid-builtin.menu-file-new");

    /// System command. A file picker dialog will be shown to the user, and an
    /// [`OPEN_FILE`] command will be sent if a file is chosen.
    ///
    /// [`OPEN_FILE`]: constant.OPEN_FILE.html
    /// [`FileDialogOptions`]: ../struct.FileDialogOptions.html
    pub const SHOW_OPEN_PANEL: Selector<FileDialogOptions> =
        Selector::new("druid-builtin.menu-file-open");

    /// Commands to open a file, must be handled by the application.
    ///
    /// [`FileInfo`]: ../struct.FileInfo.html
    pub const OPEN_FILE: Selector<FileInfo> = Selector::new("druid-builtin.open-file-path");

    /// Special command. When issued by the application, the system will show the 'save as' panel,
    /// and if a path is selected the system will issue a [`SAVE_FILE`] command
    /// with the selected path as the argument.
    ///
    /// [`SAVE_FILE`]: constant.SAVE_FILE.html
    /// [`FileDialogOptions`]: ../struct.FileDialogOptions.html
    pub const SHOW_SAVE_PANEL: Selector<FileDialogOptions> =
        Selector::new("druid-builtin.menu-file-save-as");

    /// Commands to save a file, must be handled by the application.
    ///
    /// If it carries `Some`, then the application should save to that file and store the `FileInfo` for future use.
    /// If it carries `None`, the appliaction should have recieved `Some` before and use the stored `FileInfo`.
    ///
    /// The argument, if present, should be the path where the file should be saved.
    pub const SAVE_FILE: Selector<Option<FileInfo>> = Selector::new("druid-builtin.menu-file-save");

    /// Show the print-setup window.
    pub const PRINT_SETUP: Selector<()> = Selector::new("druid-builtin.menu-file-print-setup");

    /// Show the print dialog.
    pub const PRINT: Selector<()> = Selector::new("druid-builtin.menu-file-print");

    /// Show the print preview.
    pub const PRINT_PREVIEW: Selector<()> = Selector::new("druid-builtin.menu-file-print");

    /// Cut the current selection.
    pub const CUT: Selector<()> = Selector::new("druid-builtin.menu-cut");

    /// Copy the current selection.
    pub const COPY: Selector<()> = Selector::new("druid-builtin.menu-copy");

    /// Paste.
    pub const PASTE: Selector<()> = Selector::new("druid-builtin.menu-paste");

    /// Undo.
    pub const UNDO: Selector<()> = Selector::new("druid-builtin.menu-undo");

    /// Redo.
    pub const REDO: Selector<()> = Selector::new("druid-builtin.menu-redo");
}

impl<T> Selector<T> {
    /// A selector that does nothing.
    pub const fn noop() -> Self {
        Selector::new("")
    }

    /// Create a new `Selector` with the given string.
    pub const fn new(s: &'static str) -> Self {
        Selector(s, PhantomData)
    }
}

impl<T> OneShotSelector<T> {
    /// A selector that does nothing.
    pub const fn noop() -> Self {
        OneShotSelector::new("")
    }

    /// Create a new `Selector` with the given string.
    pub const fn new(s: &'static str) -> Self {
        OneShotSelector(s, PhantomData)
    }
}

impl Command {
    /// Create a new `Command` with an argument. If you do not need
    /// an argument, `Selector` implements `Into<Command>`.
    pub fn new<T: Any>(selector: Selector<T>, arg: T) -> Self {
        Command {
            selector: selector.symbol(),
            object: Arg::Reusable(Arc::new(arg)),
        }
    }

    /// Create a new 'one-shot' `Command`.
    ///
    /// Unlike those created with `Command::new`, one-shot commands cannot
    /// be reused; their argument is consumed when it is accessed, via
    /// [`take_object`].
    ///
    /// [`take_object`]: #method.take_object
    pub fn one_shot<T: Any>(selector: OneShotSelector<T>, arg: T) -> Self {
        Command {
            selector: selector.symbol(),
            object: Arg::OneShot(Arc::new(Mutex::new(Some(Box::new(arg))))),
        }
    }

    /// Used to create a command from the types sent via an `ExtEventSink`.
    pub(crate) fn from_ext(selector: SelectorSymbol, object: Box<dyn Any + Send>) -> Self {
        let object: Box<dyn Any> = object;
        let object = Arg::Reusable(object.into());
        Command { selector, object }
    }

    pub fn is(&self, selector: impl AnySelector) -> bool {
        self.selector == selector.symbol()
    }

    /// Return a reference to this `Command`'s object, if it has one.
    ///
    /// This only works for 'reusable' commands; it does not work for commands
    /// created with [`one_shot`].
    ///
    /// [`one_shot`]: #method.one_shot
    pub fn get<T: Any>(&self, selector: Selector<T>) -> Option<&T> {
        if self.selector != selector.symbol() {
            return None;
        }
        match &self.object {
            Arg::Reusable(obj) => Some(
                obj.downcast_ref()
                    .expect("Reusable command had wrong payload type."),
            ),
            Arg::OneShot(_) => panic!("Reusable command {} carried OneShot argument.", selector),
        }
    }

    /// Attempt to take the object of a [`one-shot`] command.
    ///
    /// [`one-shot`]: #method.one_shot
    pub fn take<T: Any>(&self, selector: OneShotSelector<T>) -> Result<Box<T>, ArgumentError> {
        if self.selector != selector.symbol() {
            return Err(ArgumentError::WrongSelector);
        }
        match &self.object {
            Arg::Reusable(_) => panic!("OneShot command {} carried Reusable argument.", selector),
            Arg::OneShot(inner) => {
                let obj = inner
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or(ArgumentError::Consumed)?;
                match obj.downcast::<T>() {
                    Ok(obj) => Ok(obj),
                    Err(_) => {
                        panic!("OneShot command had wrong payload type.");
                    }
                }
            }
        }
    }
}

impl From<Selector<()>> for Command {
    fn from(selector: Selector<()>) -> Command {
        Command {
            selector: selector.symbol(),
            object: Arg::Reusable(Arc::new(())),
        }
    }
}

impl<T> std::fmt::Display for Selector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Selector(\"{}\", {})",
            self.0,
            std::any::type_name::<T>()
        )
    }
}

impl<T> std::fmt::Display for OneShotSelector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "OneShotSelector(\"{}\", {})",
            self.0,
            std::any::type_name::<T>()
        )
    }
}

impl std::fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ArgumentError::WrongSelector => write!(f, "Command had wrong selector"),
            ArgumentError::Consumed => write!(f, "One-shot command arguemnt already consumed"),
        }
    }
}

impl std::error::Error for ArgumentError {}

impl From<WindowId> for Target {
    fn from(id: WindowId) -> Target {
        Target::Window(id)
    }
}

impl From<WidgetId> for Target {
    fn from(id: WidgetId) -> Target {
        Target::Widget(id)
    }
}

impl Into<Option<Target>> for WindowId {
    fn into(self) -> Option<Target> {
        Some(Target::Window(self))
    }
}

impl Into<Option<Target>> for WidgetId {
    fn into(self) -> Option<Target> {
        Some(Target::Widget(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_object() {
        let sel: Selector<Vec<i32>> = Selector::new("my-selector");
        let objs = vec![0, 1, 2];
        // TODO: find out why this now wants a `.clone()` even tho `Selector` implements `Copy`.
        let command = Command::new(sel, objs);
        assert_eq!(command.get(sel), Some(&vec![0, 1, 2]));
    }
}
