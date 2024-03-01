// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Types related to input method editing.
//!
//! Most IME-related code is in `druid-shell`; these are helper types used
//! exclusively in `druid`.

use std::rc::Rc;

use crate::shell::text::InputHandler;
use crate::WidgetId;

/// A trait for input handlers registered by widgets.
///
/// A widget registers itself as accepting text input by calling
/// [`LifeCycleCtx::register_text_input`] while handling the
/// [`LifeCycle::WidgetAdded`] event.
///
/// The widget does not explicitly *deregister* afterwards; rather anytime
/// the widget tree changes, `druid` will call [`is_alive`] on each registered
/// `ImeHandlerRef`, and deregister those that return `false`.
///
/// [`LifeCycle::WidgetAdded`]: crate::LifeCycle::WidgetAdded
/// [`LifeCycleCtx::register_text_input`]: crate::LifeCycleCtx::register_text_input
/// [`is_alive`]: ImeHandlerRef::is_alive
pub trait ImeHandlerRef {
    /// Returns `true` if this handler is still active.
    fn is_alive(&self) -> bool;
    /// Mark the session as locked, and return a handle.
    ///
    /// The lock can be read-write or read-only, indicated by the `mutable` flag.
    ///
    /// if [`is_alive`] is `true`, this should always return `Some(_)`.
    ///
    /// [`is_alive`]: ImeHandlerRef::is_alive
    fn acquire(&self, mutable: bool) -> Option<Box<dyn InputHandler + 'static>>;
    /// Mark the session as released.
    fn release(&self) -> bool;
}

/// A type we use to keep track of which widgets are responsible for which
/// ime sessions.
#[derive(Clone)]
pub(crate) struct TextFieldRegistration {
    pub widget_id: WidgetId,
    pub document: Rc<dyn ImeHandlerRef>,
}

impl TextFieldRegistration {
    pub fn is_alive(&self) -> bool {
        self.document.is_alive()
    }
}

impl std::fmt::Debug for TextFieldRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TextFieldRegistration")
            .field("widget_id", &self.widget_id)
            .field("is_alive", &self.document.is_alive())
            .finish()
    }
}
