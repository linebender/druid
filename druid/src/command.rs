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

//! Custom commands.

use std::any::{self, Any};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{WidgetId, WindowId};

/// The identity of a [`Selector`].
///
/// [`Selector`]: struct.Selector.html
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct SelectorSymbol {
    str: &'static str,
    must_use: bool,
}

impl std::fmt::Debug for SelectorSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let must_use = if self.must_use { " (must_use)" } else { "" };
        write!(f, "{}{}", self.str, must_use)
    }
}

/// An identifier for a particular command.
///
/// This should be a unique string identifier.
/// Having multiple selectors with the same identifier but different payload
/// types is not allowed and can cause [`Command::get`] and [`get_unchecked`] to panic.
///
/// The type parameter `T` specifies the command's payload type.
/// See [`Command`] for more information.
///
/// Certain `Selector`s are defined by druid, and have special meaning
/// to the framework; these are listed in the [`druid::commands`] module.
///
/// [`Command`]: struct.Command.html
/// [`Command::get`]: struct.Command.html#method.get
/// [`get_unchecked`]: struct.Command.html#method.get_unchecked
/// [`druid::commands`]: commands/index.html
#[derive(Debug, PartialEq, Eq)]
pub struct Selector<T = ()>(SelectorSymbol, PhantomData<T>);

/// An arbitrary command.
///
/// A `Command` consists of a [`Selector`], that indicates what the command is
/// and what type of payload it carries, as well as the actual payload.
///
/// If the payload can't or shouldn't be cloned,
/// wrapping it with [`SingleUse`] allows you to `take` the payload.
/// The [`SingleUse`] docs give an example on how to do this.
///
/// Generic payloads can be achieved with `Selector<Box<dyn Any>>`.
/// In this case it could make sense to use utility functions to construct
/// such commands in order to maintain as much static typing as possible.
/// The [`EventCtx::new_window`] method is an example of this.
///
/// # Examples
/// ```
/// use druid::{Command, Selector, Target};
///
/// let selector = Selector::new("process_rows");
/// let rows = vec![1, 3, 10, 12];
/// let command = Command::new(selector, rows, Target::Auto);
///
/// assert_eq!(command.get(selector), Some(&vec![1, 3, 10, 12]));
/// ```
///
/// [`EventCtx::new_window`]: struct.EventCtx.html#method.new_window
/// [`SingleUse`]: struct.SingleUse.html
/// [`Selector`]: struct.Selector.html
#[derive(Debug, Clone)]
pub struct Command {
    symbol: SelectorSymbol,
    payload: Arc<dyn Any>,
    target: Target,
}

/// A message passed up the tree from a [`Widget`] to its ancestors.
///
/// In the course of handling an event, a [`Widget`] may change some internal
/// state that is of interest to one of its ancestors. In this case, the widget
/// may submit a [`Notification`].
///
/// In practice, a [`Notification`] is very similar to a [`Command`]; the
/// main distinction relates to delivery. [`Command`]s are delivered from the
/// root of the tree down towards the target, and this delivery occurs after
/// the originating event call has returned. [`Notification`]s are delivered *up*
/// the tree, and this occurs *during* event handling; immediately after the
/// child widget's [`event`] method returns, the notification will be delivered
/// to the child's parent, and then the parent's parent, until the notification
/// is handled.
///
/// [`Widget`]: crate::Widget
/// [`event`]: crate::Widget::event
#[derive(Clone)]
pub struct Notification {
    symbol: SelectorSymbol,
    payload: Arc<dyn Any>,
    source: WidgetId,
}

/// A wrapper type for [`Command`] payloads that should only be used once.
///
/// This is useful if you have some resource that cannot be
/// cloned, and you wish to send it to another widget.
///
/// # Examples
/// ```
/// use druid::{Command, Selector, SingleUse, Target};
///
/// struct CantClone(u8);
///
/// let selector = Selector::new("use-once");
/// let num = CantClone(42);
/// let command = Command::new(selector, SingleUse::new(num), Target::Auto);
///
/// let payload: &SingleUse<CantClone> = command.get_unchecked(selector);
/// if let Some(num) = payload.take() {
///     // now you own the data
///     assert_eq!(num.0, 42);
/// }
///
/// // subsequent calls will return `None`
/// assert!(payload.take().is_none());
/// ```
///
/// [`Command`]: struct.Command.html
pub struct SingleUse<T>(Mutex<Option<T>>);

/// The target of a [`Command`].
///
/// [`Command`]: struct.Command.html
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// The target is the top-level application.
    ///
    /// The `Command` will be delivered to all open windows, and all widgets
    /// in each window. Delivery will stop if the event is [`handled`].
    ///
    /// [`handled`]: struct.EventCtx.html#set_handled
    Global,
    /// The target is a specific window.
    ///
    /// The `Command` will be delivered to all widgets in that window.
    /// Delivery will stop if the event is [`handled`].
    ///
    /// [`handled`]: struct.EventCtx.html#set_handled
    Window(WindowId),
    /// The target is a specific widget.
    Widget(WidgetId),
    /// The target will be determined automatically.
    ///
    /// How this behaves depends on the context used to submit the command.
    /// Each `submit_command` function should have documentation about the specific behavior.
    Auto,
}

/// Commands with special meaning, defined by druid.
///
/// See [`Command`] for more info.
///
/// [`Command`]: ../struct.Command.html
pub mod sys {
    use std::any::Any;

    use super::Selector;
    use crate::{
        sub_window::{SubWindowDesc, SubWindowUpdate},
        FileDialogOptions, FileInfo, SingleUse, WidgetId, WindowConfig,
    };

    /// Quit the running application. This command is handled by the druid library.
    pub const QUIT_APP: Selector = Selector::new("druid-builtin.quit-app");

    /// Hide the application. (mac only?)
    pub const HIDE_APPLICATION: Selector = Selector::new("druid-builtin.menu-hide-application");

    /// Hide all other applications. (mac only?)
    pub const HIDE_OTHERS: Selector = Selector::new("druid-builtin.menu-hide-others");

    /// The selector for a command to create a new window.
    pub(crate) const NEW_WINDOW: Selector<SingleUse<Box<dyn Any>>> =
        Selector::new("druid-builtin.new-window");

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

    /// Apply the configuration payload to an existing window. The target should be a WindowId.
    pub const CONFIGURE_WINDOW: Selector<WindowConfig> =
        Selector::new("druid-builtin.configure-window");

    /// Display a context (right-click) menu. The payload must be the [`ContextMenu`]
    /// object to be displayed.
    ///
    /// [`ContextMenu`]: ../struct.ContextMenu.html
    pub(crate) const SHOW_CONTEXT_MENU: Selector<SingleUse<Box<dyn Any>>> =
        Selector::new("druid-builtin.show-context-menu");

    /// This is sent to the window handler to create a new sub window.
    pub(crate) const NEW_SUB_WINDOW: Selector<SingleUse<SubWindowDesc>> =
        Selector::new("druid-builtin.new-sub-window");

    /// This is sent from a WidgetPod to any attached SubWindowHosts when a data update occurs
    pub(crate) const SUB_WINDOW_PARENT_TO_HOST: Selector<SubWindowUpdate> =
        Selector::new("druid-builtin.parent_to_host");

    /// This is sent from a SubWindowHost to its parent WidgetPod after it has processed events,
    /// if that processing changed the data value.
    pub(crate) const SUB_WINDOW_HOST_TO_PARENT: Selector<Box<dyn Any>> =
        Selector::new("druid-builtin.host_to_parent");

    /// Show the application preferences.
    pub const SHOW_PREFERENCES: Selector = Selector::new("druid-builtin.menu-show-preferences");

    /// Show the application about window.
    pub const SHOW_ABOUT: Selector = Selector::new("druid-builtin.menu-show-about");

    /// Show all applications.
    pub const SHOW_ALL: Selector = Selector::new("druid-builtin.menu-show-all");

    /// Show the new file dialog.
    pub const NEW_FILE: Selector = Selector::new("druid-builtin.menu-file-new");

    /// When submitted by the application, a file picker dialog will be shown to the user,
    /// and an [`OPEN_FILE`] command will be sent if a path is chosen.
    ///
    /// [`OPEN_FILE`]: constant.OPEN_FILE.html
    pub const SHOW_OPEN_PANEL: Selector<FileDialogOptions> =
        Selector::new("druid-builtin.menu-file-open");

    /// Sent when the user cancels an open file panel.
    pub const OPEN_PANEL_CANCELLED: Selector = Selector::new("druid-builtin.open-panel-cancelled");

    /// Open a path, must be handled by the application.
    ///
    /// [`FileInfo`]: ../struct.FileInfo.html
    pub const OPEN_FILE: Selector<FileInfo> = Selector::new("druid-builtin.open-file-path");

    /// When submitted by the application, the system will show the 'save as' panel,
    /// and if a path is selected the system will issue a [`SAVE_FILE`] command
    /// with the selected path as the payload.
    ///
    /// [`SAVE_FILE`]: constant.SAVE_FILE.html
    pub const SHOW_SAVE_PANEL: Selector<FileDialogOptions> =
        Selector::new("druid-builtin.menu-file-save-as");

    /// Sent when the user cancels a save file panel.
    pub const SAVE_PANEL_CANCELLED: Selector = Selector::new("druid-builtin.save-panel-cancelled");

    /// Save the current path.
    ///
    /// The application should save its data, to a path that should be determined by the
    /// application. Usually, this will be the most recent path provided by a [`SAVE_FILE_AS`]
    /// or [`OPEN_FILE`] command.
    pub const SAVE_FILE: Selector<()> = Selector::new("druid-builtin.save-file");

    /// Save to a given location.
    ///
    /// This command is emitted by druid whenever a save file dialog successfully completes. The
    /// application should save its data to the path proved, and should store the path in order to
    /// handle [`SAVE_FILE`] commands in the future.
    ///
    /// The path might be a file or a directory, so always check whether it matches your
    /// expectations.
    pub const SAVE_FILE_AS: Selector<FileInfo> = Selector::new("druid-builtin.save-file-as");

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

    /// Text input state has changed, and we need to notify the platform.
    pub(crate) const INVALIDATE_IME: Selector<ImeInvalidation> =
        Selector::new("druid-builtin.invalidate-ime");

    /// A change that has occured to text state, and needs to be
    /// communicated to the platform.
    pub(crate) struct ImeInvalidation {
        pub widget: WidgetId,
        pub event: crate::shell::text::Event,
    }
}

impl Selector<()> {
    /// A selector that does nothing.
    pub const NOOP: Selector = Selector::new("");

    /// Turns this into a command with the specified [`Target`].
    ///
    /// [`Target`]: enum.Target.html
    pub fn to(self, target: impl Into<Target>) -> Command {
        Command::from(self).to(target.into())
    }
}

impl<T> Selector<T> {
    /// Create a new `Selector` with the given string.
    pub const fn new(str: &'static str) -> Selector<T> {
        Selector(
            SelectorSymbol {
                str,
                must_use: false,
            },
            PhantomData,
        )
    }

    /// Create a `Selector` that must be used.
    pub const fn must_use(str: &'static str) -> Selector<T> {
        Selector(
            SelectorSymbol {
                str,
                must_use: true,
            },
            PhantomData,
        )
    }

    /// Returns the `SelectorSymbol` identifying this `Selector`.
    pub(crate) const fn symbol(self) -> SelectorSymbol {
        self.0
    }
}

impl<T: Any> Selector<T> {
    /// Convenience method for [`Command::new`] with this selector.
    ///
    /// If the payload is `()` there is no need to call this,
    /// as `Selector<()>` implements `Into<Command>`.
    ///
    /// By default, the command will have [`Target::Auto`].
    /// The [`Command::to`] method can be used to override this.
    ///
    /// [`Command::new`]: struct.Command.html#method.new
    /// [`Command::to`]: struct.Command.html#method.to
    /// [`Target::Auto`]: enum.Target.html#variant.Auto
    pub fn with(self, payload: T) -> Command {
        Command::new(self, payload, Target::Auto)
    }
}

impl Command {
    /// Create a new `Command` with a payload and a [`Target`].
    ///
    /// [`Selector::with`] can be used to create `Command`s more conveniently.
    ///
    /// If you do not need a payload, [`Selector`] implements `Into<Command>`.
    ///
    /// [`Selector`]: struct.Selector.html
    /// [`Selector::with`]: struct.Selector.html#method.with
    /// [`Target`]: enum.Target.html
    pub fn new<T: Any>(selector: Selector<T>, payload: T, target: impl Into<Target>) -> Self {
        Command {
            symbol: selector.symbol(),
            payload: Arc::new(payload),
            target: target.into(),
        }
    }

    /// Used to create a `Command` from the types sent via an `ExtEventSink`.
    pub(crate) fn from_ext(symbol: SelectorSymbol, payload: Box<dyn Any>, target: Target) -> Self {
        Command {
            symbol,
            payload: payload.into(),
            target,
        }
        .default_to(Target::Global)
    }

    /// Checks if this command must be used.
    pub fn must_be_used(&self) -> bool {
        self.symbol.must_use
    }

    /// A helper method for creating a `Notification` from a `Command`.
    ///
    /// This is slightly icky; it lets us do `SOME_SELECTOR.with(SOME_PAYLOAD)`
    /// (which generates a command) and then privately convert it to a
    /// notification.
    pub(crate) fn into_notification(self, source: WidgetId) -> Notification {
        Notification {
            symbol: self.symbol,
            payload: self.payload,
            source,
        }
    }

    /// Set the `Command`'s [`Target`].
    ///
    /// [`Command::target`] can be used to get the current [`Target`].
    ///
    /// [`Command::target`]: #method.target
    /// [`Target`]: enum.Target.html
    pub fn to(mut self, target: impl Into<Target>) -> Self {
        self.target = target.into();
        self
    }

    /// Make the `Command` must use.
    ///
    /// this will log warning if this `Command` is not handled.
    pub fn must_use(mut self, must_use: bool) -> Self {
        self.symbol.must_use = must_use;
        self
    }

    /// Set the correct default target when target is `Auto`.
    pub(crate) fn default_to(mut self, target: Target) -> Self {
        self.target.default(target);
        self
    }

    /// Returns the `Command`'s [`Target`].
    ///
    /// [`Command::to`] can be used to change the [`Target`].
    ///
    /// [`Command::to`]: #method.to
    /// [`Target`]: enum.Target.html
    pub fn target(&self) -> Target {
        self.target
    }

    /// Returns `true` if `self` matches this `selector`.
    pub fn is<T>(&self, selector: Selector<T>) -> bool {
        self.symbol == selector.symbol()
    }

    /// Returns `Some(&T)` (this `Command`'s payload) if the selector matches.
    ///
    /// Returns `None` when `self.is(selector) == false`.
    ///
    /// Alternatively you can check the selector with [`is`] and then use [`get_unchecked`].
    ///
    /// # Panics
    ///
    /// Panics when the payload has a different type, than what the selector is supposed to carry.
    /// This can happen when two selectors with different types but the same key are used.
    ///
    /// [`is`]: #method.is
    /// [`get_unchecked`]: #method.get_unchecked
    pub fn get<T: Any>(&self, selector: Selector<T>) -> Option<&T> {
        if self.symbol == selector.symbol() {
            Some(self.payload.downcast_ref().unwrap_or_else(|| {
                panic!(
                    "The selector {:?} exists twice with different types. See druid::Command::get for more information",
                    selector.symbol()
                );
            }))
        } else {
            None
        }
    }

    /// Returns a reference to this `Command`'s payload.
    ///
    /// If the selector has already been checked with [`is`], then `get_unchecked` can be used safely.
    /// Otherwise you should use [`get`] instead.
    ///
    /// # Panics
    ///
    /// Panics when `self.is(selector) == false`.
    ///
    /// Panics when the payload has a different type, than what the selector is supposed to carry.
    /// This can happen when two selectors with different types but the same key are used.
    ///
    /// [`is`]: #method.is
    /// [`get`]: #method.get
    pub fn get_unchecked<T: Any>(&self, selector: Selector<T>) -> &T {
        self.get(selector).unwrap_or_else(|| {
            panic!(
                "Expected selector {:?} but the command was {:?}.",
                selector.symbol(),
                self.symbol
            )
        })
    }
}

impl Notification {
    /// Returns `true` if `self` matches this [`Selector`].
    pub fn is<T>(&self, selector: Selector<T>) -> bool {
        self.symbol == selector.symbol()
    }

    /// Returns the payload for this [`Selector`], if the selector matches.
    ///
    /// # Panics
    ///
    /// Panics when the payload has a different type, than what the selector
    /// is supposed to carry. This can happen when two selectors with different
    /// types but the same key are used.
    ///
    /// [`is`]: #method.is
    pub fn get<T: Any>(&self, selector: Selector<T>) -> Option<&T> {
        if self.symbol == selector.symbol() {
            Some(self.payload.downcast_ref().unwrap_or_else(|| {
                panic!(
                    "The selector {:?} exists twice with different types. \
                    See druid::Command::get for more information",
                    selector.symbol()
                );
            }))
        } else {
            None
        }
    }

    /// The [`WidgetId`] of the [`Widget`] that sent this [`Notification`].
    ///
    /// [`Widget`]: crate::Widget
    pub fn source(&self) -> WidgetId {
        self.source
    }
}

impl<T: Any> SingleUse<T> {
    /// Create a new single-use payload.
    pub fn new(data: T) -> Self {
        SingleUse(Mutex::new(Some(data)))
    }

    /// Takes the value, leaving a None in its place.
    pub fn take(&self) -> Option<T> {
        self.0.lock().unwrap().take()
    }
}

impl From<Selector> for Command {
    fn from(selector: Selector) -> Command {
        Command {
            symbol: selector.symbol(),
            payload: Arc::new(()),
            target: Target::Auto,
        }
    }
}

impl<T> std::fmt::Display for Selector<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Selector({:?}, {})", self.0, any::type_name::<T>())
    }
}

// This has do be done explicitly, to avoid the Copy bound on `T`.
// See https://doc.rust-lang.org/std/marker/trait.Copy.html#how-can-i-implement-copy .
impl<T> Copy for Selector<T> {}
impl<T> Clone for Selector<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Target {
    /// If `self` is `Auto` it will be replaced with `target`.
    pub(crate) fn default(&mut self, target: Target) {
        if self == &Target::Auto {
            *self = target;
        }
    }
}

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

impl From<WindowId> for Option<Target> {
    fn from(id: WindowId) -> Self {
        Some(Target::Window(id))
    }
}

impl From<WidgetId> for Option<Target> {
    fn from(id: WidgetId) -> Self {
        Some(Target::Widget(id))
    }
}

impl std::fmt::Debug for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Notification: Selector {:?} from {:?}",
            self.symbol, self.source
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_env_log::test;

    #[test]
    fn get_payload() {
        let sel = Selector::new("my-selector");
        let payload = vec![0, 1, 2];
        let command = Command::new(sel, payload, Target::Auto);
        assert_eq!(command.get(sel), Some(&vec![0, 1, 2]));
    }

    #[test]
    fn selector_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Selector>();
    }
}
