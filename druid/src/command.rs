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
use std::sync::{Arc, Mutex};

use crate::{WidgetId, WindowId};

/// An identifier for a particular command.
///
/// This should be a unique string identifier. Certain `Selector`s are defined
/// by druid, and have special meaning to the framework; these are listed in the
/// [`druid::commands`] module.
///
/// [`druid::commands`]: commands/index.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector(&'static str);

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
/// assert_eq!(command.get_object(), Ok(&vec![1, 3, 10, 12]));
/// ```
///
/// [`Command::new`]: #method.new
/// [`Command::get_object`]: #method.get_object
/// [`Selector`]: struct.Selector.html
#[derive(Debug, Clone)]
pub struct Command {
    /// The command's `Selector`.
    pub selector: Selector,
    object: Option<Arg>,
}

#[derive(Debug, Clone)]
enum Arg {
    Reusable(Arc<dyn Any>),
    OneShot(Arc<Mutex<Option<Box<dyn Any>>>>),
}

/// Errors that can occur when attempting to retrieve the a command's argument.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentError {
    /// The command did not have an argument.
    NoArgument,
    /// The argument was expected to be reusable and wasn't, or vice-versa.
    WrongVariant,
    /// The argument could not be downcast to the specified type.
    IncorrectType,
    /// The one-shot argument has already been taken.
    Consumed,
}

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
    use super::Selector;

    /// Quit the running application. This command is handled by the druid library.
    pub const QUIT_APP: Selector = Selector::new("druid-builtin.quit-app");

    /// Hide the application. (mac only?)
    pub const HIDE_APPLICATION: Selector = Selector::new("druid-builtin.menu-hide-application");

    /// Hide all other applications. (mac only?)
    pub const HIDE_OTHERS: Selector = Selector::new("druid-builtin.menu-hide-others");

    /// The selector for a command to create a new window.
    pub const NEW_WINDOW: Selector = Selector::new("druid-builtin.new-window");

    /// The selector for a command to close a window.
    ///
    /// The command must target a specific window.
    /// When calling `submit_command` on a `Widget`s context, passing `None` as target
    /// will automatically target the window containing the widget.
    pub const CLOSE_WINDOW: Selector = Selector::new("druid-builtin.close-window");

    /// Close all windows.
    pub const CLOSE_ALL_WINDOWS: Selector = Selector::new("druid-builtin.close-all-windows");

    /// The selector for a command to bring a window to the front, and give it focus.
    ///
    /// The command must target a specific window.
    /// When calling `submit_command` on a `Widget`s context, passing `None` as target
    /// will automatically target the window containing the widget.
    pub const SHOW_WINDOW: Selector = Selector::new("druid-builtin.show-window");

    /// Display a context (right-click) menu. The argument must be the [`ContextMenu`].
    /// object to be displayed.
    ///
    /// [`ContextMenu`]: ../struct.ContextMenu.html
    pub const SHOW_CONTEXT_MENU: Selector = Selector::new("druid-builtin.show-context-menu");

    /// The selector for a command to set the window's menu. The argument should
    /// be a [`MenuDesc`] object.
    ///
    /// [`MenuDesc`]: ../struct.MenuDesc.html
    pub const SET_MENU: Selector = Selector::new("druid-builtin.set-menu");

    /// Show the application preferences.
    pub const SHOW_PREFERENCES: Selector = Selector::new("druid-builtin.menu-show-preferences");

    /// Show the application about window.
    pub const SHOW_ABOUT: Selector = Selector::new("druid-builtin.menu-show-about");

    /// Show all applications.
    pub const SHOW_ALL: Selector = Selector::new("druid-builtin.menu-show-all");

    /// Show the new file dialog.
    pub const NEW_FILE: Selector = Selector::new("druid-builtin.menu-file-new");

    /// System command. A file picker dialog will be shown to the user, and an
    /// [`OPEN_FILE`] command will be sent if a file is chosen.
    ///
    /// The argument should be a [`FileDialogOptions`] struct.
    ///
    /// [`OPEN_FILE`]: constant.OPEN_FILE.html
    /// [`FileDialogOptions`]: ../struct.FileDialogOptions.html
    pub const SHOW_OPEN_PANEL: Selector = Selector::new("druid-builtin.menu-file-open");

    /// Open a file.
    ///
    /// The argument must be a [`FileInfo`] object for the file to be opened.
    ///
    /// [`FileInfo`]: ../struct.FileInfo.html
    pub const OPEN_FILE: Selector = Selector::new("druid-builtin.open-file-path");

    /// Special command. When issued, the system will show the 'save as' panel,
    /// and if a path is selected the system will issue a [`SAVE_FILE`] command
    /// with the selected path as the argument.
    ///
    /// The argument should be a [`FileDialogOptions`] object.
    ///
    /// [`SAVE_FILE`]: constant.SAVE_FILE.html
    /// [`FileDialogOptions`]: ../struct.FileDialogOptions.html
    pub const SHOW_SAVE_PANEL: Selector = Selector::new("druid-builtin.menu-file-save-as");

    /// Save the current file.
    ///
    /// The argument, if present, should be the path where the file should be saved.
    pub const SAVE_FILE: Selector = Selector::new("druid-builtin.menu-file-save");

    /// Show the print-setup window.
    pub const PRINT_SETUP: Selector = Selector::new("druid-builtin.menu-file-print-setup");

    /// Show the print dialog.
    pub const PRINT: Selector = Selector::new("druid-builtin.menu-file-print");

    /// Show the print preview.
    pub const PRINT_PREVIEW: Selector = Selector::new("druid-builtin.menu-file-print");

    /// Cut the current selection.
    pub const CUT: Selector = Selector::new("druid-builtin.menu-cut");

    /// Copy the current selection.
    pub const COPY: Selector = Selector::new("druid-builtin.menu-copy");

    /// Paste.
    pub const PASTE: Selector = Selector::new("druid-builtin.menu-paste");

    /// Undo.
    pub const UNDO: Selector = Selector::new("druid-builtin.menu-undo");

    /// Redo.
    pub const REDO: Selector = Selector::new("druid-builtin.menu-redo");
}

impl Selector {
    /// A selector that does nothing.
    pub const NOOP: Selector = Selector::new("");

    /// Create a new `Selector` with the given string.
    pub const fn new(s: &'static str) -> Selector {
        Selector(s)
    }
}

impl Command {
    /// Create a new `Command` with an argument. If you do not need
    /// an argument, `Selector` implements `Into<Command>`.
    pub fn new(selector: Selector, arg: impl Any) -> Self {
        Command {
            selector,
            object: Some(Arg::Reusable(Arc::new(arg))),
        }
    }

    /// Create a new 'one-shot' `Command`.
    ///
    /// Unlike those created with `Command::new`, one-shot commands cannot
    /// be reused; their argument is consumed when it is accessed, via
    /// [`take_object`].
    ///
    /// [`take_object`]: #method.take_object
    pub fn one_shot(selector: Selector, arg: impl Any) -> Self {
        Command {
            selector,
            object: Some(Arg::OneShot(Arc::new(Mutex::new(Some(Box::new(arg)))))),
        }
    }

    /// Used to create a command from the types sent via an `ExtEventSink`.
    pub(crate) fn from_ext(selector: Selector, object: Option<Box<dyn Any + Send>>) -> Self {
        let object: Option<Box<dyn Any>> = object.map(|obj| obj as Box<dyn Any>);
        let object = object.map(|o| Arg::Reusable(o.into()));
        Command { selector, object }
    }

    /// Return a reference to this `Command`'s object, if it has one.
    ///
    /// This only works for 'reusable' commands; it does not work for commands
    /// created with [`one_shot`].
    ///
    /// [`one_shot`]: #method.one_shot
    pub fn get_object<T: Any>(&self) -> Result<&T, ArgumentError> {
        match self.object.as_ref() {
            Some(Arg::Reusable(o)) => o.downcast_ref().ok_or(ArgumentError::IncorrectType),
            Some(Arg::OneShot(_)) => Err(ArgumentError::WrongVariant),
            None => Err(ArgumentError::NoArgument),
        }
    }

    /// Attempt to take the object of a [`one-shot`] command.
    ///
    /// [`one-shot`]: #method.one_shot
    pub fn take_object<T: Any>(&self) -> Result<Box<T>, ArgumentError> {
        match self.object.as_ref() {
            Some(Arg::Reusable(_)) => Err(ArgumentError::WrongVariant),
            Some(Arg::OneShot(inner)) => {
                let obj = inner
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or(ArgumentError::Consumed)?;
                match obj.downcast::<T>() {
                    Ok(obj) => Ok(obj),
                    Err(obj) => {
                        inner.lock().unwrap().replace(obj);
                        Err(ArgumentError::IncorrectType)
                    }
                }
            }
            None => Err(ArgumentError::NoArgument),
        }
    }
}

impl From<Selector> for Command {
    fn from(selector: Selector) -> Command {
        Command {
            selector,
            object: None,
        }
    }
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Selector('{}')", self.0)
    }
}

impl std::fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ArgumentError::NoArgument => write!(f, "Command has no argument"),
            ArgumentError::IncorrectType => write!(f, "Downcast failed: wrong concrete type"),
            ArgumentError::Consumed => write!(f, "One-shot command arguemnt already consumed"),
            ArgumentError::WrongVariant => write!(
                f,
                "Incorrect access method for argument type; \
                 check Command::one_shot docs for more detail."
            ),
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
        let sel = Selector::new("my-selector");
        let objs = vec![0, 1, 2];
        let command = Command::new(sel, objs);
        assert_eq!(command.get_object(), Ok(&vec![0, 1, 2]));
    }
}
