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
use std::sync::Arc;

/// An identifier for a particular command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector(&'static str);

/// An arbitrary command.
///
/// A `Command` consists of a `Selector`, that indicates what the command is,
/// and an optional argument, that can be used to pass arbitrary data.
///
/// # Examples
/// ```
/// use druid::{Command, Selector};
///
/// let selector = Selector::new("process_rows");
/// let rows = vec![1, 3, 10, 12];
/// let command = Command::new(selector, rows);
///
/// assert_eq!(command.get_object(), Some(&vec![1, 3, 10, 12]));
/// ```
#[derive(Debug, Clone)]
pub struct Command {
    pub selector: Selector,
    object: Option<Arc<dyn Any>>,
}

impl Selector {
    /// A selector that does nothing.
    pub const NOOP: Selector = Selector::new("");

    /// Quit the running application. This command is handled by the druid library.
    pub const QUIT_APP: Selector = Selector::new("druid-builtin.quit-app");

    /// Hide the application. (mac only?)
    pub const HIDE_APPLICATION: Selector = Selector::new("druid-builtin.menu-hide-application");

    /// Hide all other applications. (mac only?)
    pub const HIDE_OTHERS: Selector = Selector::new("druid-builtin.menu-hide-others");

    /// The selector for a command to create a new window.
    pub const NEW_WINDOW: Selector = Selector::new("druid-builtin.new-window");

    /// The selector for a command to close a window. The command's argument
    /// should be the id of the window to close.
    pub const CLOSE_WINDOW: Selector = Selector::new("druid-builtin.close-window");

    /// The selector for a command to set the window's menu. The argument should
    /// be a [`Menu`] object.
    ///
    /// [`Menu`]: struct.Menu.html
    pub const SET_MENU: Selector = Selector::new("druid-builtin.set-menu");

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
            object: Some(Arc::new(arg)),
        }
    }

    /// Return a reference to this command's object, if it has one.
    pub fn get_object<T: Any>(&self) -> Option<&T> {
        match self.object.as_ref() {
            None => None,
            Some(obj) => obj.downcast_ref::<T>(),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_object() {
        let sel = Selector::new("my-selector");
        let objs = vec![0, 1, 2];
        let command = Command::new(sel, objs);
        assert_eq!(command.get_object(), Some(&vec![0, 1, 2]));
    }
}
