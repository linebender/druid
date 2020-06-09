// Copyright 2020 The xi-editor Authors.
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

//! Alert dialogs.

use std::borrow::Cow;

/// A token that uniquely identifies an alert dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct AlertToken(usize);

/// Contains the result of the alert after it was closed.
#[derive(Debug, Clone)]
pub struct AlertResponse {
    token: AlertToken,
    button: Option<AlertButton>,
}

/// Options for alert dialogs.
///
/// An alert dialog is a modal dialog that provides the user with some information
/// and optionally asks the user to make some decision.
///
/// # Button order
///
/// The order of buttons across button types is determined automatically
/// based on the guidelines of the specific platform.
///
/// # User experience
///
/// You should try to minimize the use of alerts. Alerts interrupt whatever task
/// the user is currently doing. Only use alerts for time sensitive information.
/// If there is some piece of information that the user must know immediately,
/// e.g. that storage space is running very low and will soon cause a critical error,
/// or if the storage space has already depleted and the critcial error has happened.
/// Alerts are also useful when there is some decision that the user must make
/// and the application can't continue without making that decision. Even so
/// you should think hard when designing your application whether you can have
/// reasonable default behavior which can happen without alerts. It's better to
/// offer an *undo* feature than to ask for confirmation on deletion.
///
/// The fewer alerts the user encounters, the better their experience.
///
/// # Modality
///
/// Alerts can operate in two different modality modes. By default
/// they are window scoped. Alternatively they can be application scoped.
///
/// While an alert dialog is open it takes interaction priority in its modality scope
/// and the user won't be able to continue interacting with anything else.
/// In window scope this means the user can't interact with the parent window
/// and in application scope this means the user can't interact with any other window.
///
/// # Multiple alerts
///
/// There is no guaranteed behavior when multiple alert dialogs are requested with
/// overlapping modality scope. Depending on platform specifics, all the alerts
/// might be visible or just some of them. Regardless of visibility, only one of
/// the alert dialogs will be interactable. When that alert dialog is closed,
/// another one will become interactable.
///
/// If the modality scopes don't overlap, i.e. when multiple alerts are originating
/// from different windows and they are all window scoped, all the alerts can be
/// interacted with and they won't block eachother.
#[derive(Debug, Clone)]
pub struct AlertOptions {
    /// Whether the alert is app-modal.
    pub(crate) app_modal: bool,
    /// The context of the alert.
    pub(crate) context: String,
    /// The primary message of the alert.
    pub(crate) message: String,
    /// The supplemental text of the alert.
    pub(crate) description: String,
    /// The icon of the alert.
    pub(crate) icon: Option<AlertIcon>,
    /// The primary button of the alert.
    pub(crate) primary: AlertButton,
    /// The cancel button of the alert.
    pub(crate) cancel: Option<AlertButton>,
    /// The alternative buttons of the alert.
    pub(crate) alternatives: Vec<AlertButton>,
}

/// Alert dialog icon.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertIcon {
    /// The alert dialog is providing important time sensitive information.
    Information,
    /// The alert dialog is warning about a potential future problem.
    Warning,
    /// The alert dialog is informing of a critical problem that has already happened.
    Error,
}

/// A specific button for the alert dialog.
///
/// If the alert is just reporting information and it only needs a single button,  
/// then you should use a positive button with the label **OK** or a translation of it.
///
/// The cancel button should always be labeled **Cancel** or a translation of it.
///
/// In all other cases you should give a short but descriptive label that includes a verb.
/// The label should let the user know what will happen when they click the button.
/// Assume that the user didn't actually read the alert message. This assumption ends up
/// true more often than not, because alerts by design appear unexpectedly while the user
/// is trying to accomplish some other task.
///
/// Labels like **Send message**, **View all**, **Reconnect**, **Remind me tomorrow**, and
/// **Delete file** make it much more clear what will happen compared to a generic **OK**,
/// **Confirm**, or **Yes**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertButton {
    pub(crate) label: Cow<'static, str>,
}

impl AlertToken {
    /// A token that does not correspond to any alert.
    pub const INVALID: AlertToken = AlertToken(0);

    /// Create a new token with the specified `id`.
    pub(crate) const fn new(id: usize) -> AlertToken {
        AlertToken(id)
    }
}

impl AlertResponse {
    /// Create a new alert response.
    pub(crate) fn new(token: AlertToken, button: Option<AlertButton>) -> AlertResponse {
        AlertResponse { token, button }
    }

    /// Returns the [`AlertToken`] that identifies the alert.
    ///
    /// [`AlertToken`]: struct.AlertToken.html
    #[inline]
    pub fn token(&self) -> AlertToken {
        self.token
    }

    /// Returns the [`AlertButton`] that closed the alert.
    ///
    /// # Canceled
    ///
    /// This will return `None` if the alert was canceled.
    ///
    /// Keep in mind that the platform may forcefully cancel the alert, e.g. when
    /// the parent window is closed, even when the alert dialog has no cancel button.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    #[inline]
    pub fn button(&self) -> Option<&AlertButton> {
        self.button.as_ref()
    }
}

impl AlertButton {
    /// A primary button with the English label **OK**.
    pub const OK: AlertButton = AlertButton::new("OK");
    /// A cancel button with the English label **Cancel**.
    pub const CANCEL: AlertButton = AlertButton::new("Cancel");

    /// Create a new `const` button with the specified `label`.
    ///
    /// You should give a short but descriptive label like **Send message** that lets
    /// the user know what will happen when they click the button.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    pub const fn new(label: &'static str) -> AlertButton {
        AlertButton {
            label: Cow::Borrowed(label),
        }
    }

    /// Create a new button with the specified `label` at runtime.
    ///
    /// This is the runtime version of [`new`] that allows you to use
    /// runtime generated labels, e.g. dynamic personalization or translation.
    ///
    /// You should give a short but descriptive label like **Send message** that lets
    /// the user know what will happen when they click the button.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`new`]: #method.new
    /// [`AlertButton`]: struct.AlertButton.html
    pub fn dynamic(label: impl Into<String>) -> AlertButton {
        AlertButton {
            label: Cow::Owned(label.into()),
        }
    }
}

impl Default for AlertOptions {
    /// Create a default set of alert options.
    fn default() -> AlertOptions {
        AlertOptions {
            app_modal: false,
            context: Default::default(),
            message: Default::default(),
            description: Default::default(),
            icon: None,
            primary: AlertButton::OK,
            cancel: None,
            alternatives: Vec::new(),
        }
    }
}

impl AlertOptions {
    /// Create a new set of alert options.
    pub fn new() -> Self {
        AlertOptions::default()
    }

    /// Create a new set of alert options with [`AlertIcon::Information`].
    ///
    /// Use this when the alert dialog is providing important time sensitive information.
    ///
    /// This function is equivalent to calling [`icon`] with [`AlertIcon::Information`].
    ///
    /// [`icon`]: #method.icon
    /// [`AlertIcon::Information`]: enum.AlertIcon.html#variant.Information
    pub fn information() -> Self {
        AlertOptions {
            icon: Some(AlertIcon::Information),
            ..Default::default()
        }
    }

    /// Create a new set of alert options with [`AlertIcon::Warning`].
    ///
    /// Use this when the alert dialog is warning about a potential future problem.
    ///
    /// This function is equivalent to calling [`icon`] with [`AlertIcon::Warning`].
    ///
    /// [`icon`]: #method.icon
    /// [`AlertIcon::Warning`]: enum.AlertIcon.html#variant.Warning
    pub fn warning() -> Self {
        AlertOptions {
            icon: Some(AlertIcon::Warning),
            ..Default::default()
        }
    }

    /// Create a new set of alert options with [`AlertIcon::Error`].
    ///
    /// Use this when the alert dialog is informing of a critical problem that has already happened.
    ///
    /// This function is equivalent to calling [`icon`] with [`AlertIcon::Error`].
    ///
    /// [`icon`]: #method.icon
    /// [`AlertIcon::Error`]: enum.AlertIcon.html#variant.Error
    pub fn error() -> Self {
        AlertOptions {
            icon: Some(AlertIcon::Error),
            ..Default::default()
        }
    }

    /// Set the icon of the alert.
    ///
    /// This should accurately represent your primary message.
    /// Read [`AlertIcon`] for more information about icons.
    ///
    /// This is shown on the alert dialog next to your primary message.
    ///
    /// Setting this to `None` means there will not be any icon.
    ///
    /// [`AlertIcon`]: enum.AlertIcon.html
    pub fn icon(mut self, icon: impl Into<Option<AlertIcon>>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the context of the alert.
    ///
    /// This should be the name of the task that this alert is related to,
    /// or a related component name, or at the very least your application name.
    ///
    /// This gets shown in the alert dialog title bar.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    /// Set the primary message of the alert.
    ///
    /// This should be the key statement or question to the user.
    /// Keep it short and to the point. For more details use [`description`].
    ///
    /// This is the most visually emphasized text of the alert.
    ///
    /// [`description`]: #method.description
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Set the supplemental text of the alert.
    ///
    /// This should explain your primary message in more detail.
    ///
    /// This is shown as regular text under the primary message.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the primary button of the alert.
    ///
    /// The primary button is the hero of the alert dialog. It should be the button
    /// that agrees with the message of the alert, and it should be the one that is
    /// most likely to be chosen by the user.
    ///
    /// This defaults to the English [`AlertButton::OK`].
    ///
    /// If the alert has more than one button then you should change this to something
    /// more descriptive of the action that will be taken.
    ///
    /// Read [`AlertButton`] for more information about buttons.
    ///
    /// # Example
    ///
    /// When a user attempts to close a new document, the primary button should be **Save**.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    /// [`AlertButton::OK`]: struct.AlertButton.html#associatedconstant.OK
    pub fn primary(mut self, button: AlertButton) -> Self {
        self.primary = button;
        self
    }

    /// Add an alternative button to the alert.
    ///
    /// Alternative buttons are a way to provide the user with multiple choices.
    /// These are usually not the choices that the user will make, but in the right
    /// circumstances can be very valuable.
    ///
    /// Alternative buttons can provide the user a way to disagree with the message
    /// of the alert but still proceed. Agree or not, alternative buttons provide
    /// ways to proceed that differ from the primary button.
    ///
    /// You can have multiple alternative buttons by calling this method multiple times.
    ///
    /// Keep the total number of buttons on the alert dialog low, almost never
    /// present more than three including the [`primary`] and [`cancel`] buttons.
    /// Having more buttons is supported, but will look quirky and will create
    /// a worse user experience by introducing an overly complex situation.
    ///
    /// Read [`AlertButton`] for more information about buttons.
    ///
    /// # Example
    ///
    /// When a user attempts to close a new document, an alternative button should be **Delete**.
    ///
    /// [`primary`]: #method.primary
    /// [`cancel`]: #method.cancel
    /// [`AlertButton`]: struct.AlertButton.html
    pub fn alternative(mut self, button: AlertButton) -> Self {
        self.alternatives.push(button);
        self
    }

    /// Set the cancel button of the alert.
    ///
    /// The cancel button is used by the user to show a desire to not answer at all
    /// and instead cancel the task which initiated the alert.
    ///
    /// Specifying a cancel button will also enable other platform provided ways to
    /// cancel the alert, e.g. closing the alert window or pressing the escape key.
    ///
    /// The cancel button should always be [`AlertButton::CANCEL`] or a translation of it.
    ///
    /// This defaults to `None` which omits the cancel button. Keep in mind that the platform
    /// may still forcefully cancel the alert anyway, e.g. when the parent window is closed.
    ///
    /// Read [`AlertButton`] for more information about buttons.
    ///
    /// # Example
    ///
    /// When a user attempts to close a new document, the cancel button should be **Cancel**.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    /// [`AlertButton::CANCEL`]: struct.AlertButton.html#associatedconstant.CANCEL
    pub fn cancel(mut self, button: impl Into<Option<AlertButton>>) -> Self {
        self.cancel = button.into();
        self
    }

    /// Set the cancel button of the alert to [`AlertButton::CANCEL`].
    ///
    /// This is a convenience method that is equal to calling [`cancel`]
    /// with [`AlertButton::CANCEL`]. Read [`cancel`] for more information.
    ///
    /// [`AlertButton::CANCEL`]: struct.AlertButton.html#associatedconstant.CANCEL
    /// [`cancel`]: #method.cancel
    pub fn cancelable(mut self) -> Self {
        self.cancel = Some(AlertButton::CANCEL);
        self
    }

    /// Set the alert dialog modality to be application scoped.
    ///
    /// By default the alert dialog modality is window scoped, which means
    /// that the user will not be able to interact with the parent window of the alert.
    ///
    /// An app-modal alert will prevent the user from interacting with
    /// any window of the application.
    /// 
    /// [Read more about modality.]
    /// 
    /// [Read more about modality.]: #modality
    pub fn app_modal(mut self) -> Self {
        self.app_modal = true;
        self
    }
}
