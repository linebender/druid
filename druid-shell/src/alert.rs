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

use crate::util::ConstString;

/// A token that uniquely identifies an alert dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct AlertToken(usize);

/// Contains the result of the alert after it was closed.
#[derive(Debug, Clone)]
pub struct AlertResponse {
    token: AlertToken,
    button: AlertButton,
}

/// Options for alert dialogs.
///
/// An alert dialog is a modal dialog that provides the user with some information
/// and optionally asks the user to make some decision.
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
/// While an alert dialog is open it takes interaction priority and the user won't be
/// able to continue interacting with the parent window while the alert is open.
///
/// Multiple alert dialogs can be open at once, but only the most recent one will be interactable.
/// When that alert dialog is closed, the next most recent will become interactive.
#[derive(Debug, Clone, Default)]
pub struct AlertOptions {
    /// The context of the alert.
    pub(crate) context: String,
    /// The primary message of the alert.
    pub(crate) message: String,
    /// The supplemental text of the alert.
    pub(crate) description: String,
    /// The icon of the alert.
    pub(crate) icon: Option<AlertIcon>,
    /// The buttons to be shown on the alert.
    pub(crate) buttons: Vec<AlertButton>,
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

/// Alert dialog button type.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertButtonType {
    /// Positive buttons are used for choices that agree with the primary alert message.
    Positive,
    /// Negative buttons are used for choices that disagree with the primary alert message.
    Negative,
    /// Cancel buttons reflect the user's desire to not answer at all
    /// and instead cancel the task which initiated the alert.
    Cancel,
}

/// A specific button for the alert dialog.
///
/// There are three types of buttons:
/// - **Positive** - for choices that agree with the primary alert message.
/// - **Negative** - for choices that disagree with the primary alert message.
/// - **Cancel** - used by the user to show a desire to not answer at all
///   and instead cancel the task which initiated the alert.
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
    pub(crate) button_type: AlertButtonType,
    pub(crate) label: ConstString,
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
    pub(crate) fn new(token: AlertToken, button: AlertButton) -> AlertResponse {
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
    /// # Canceled by the platform
    ///
    /// If the platform cancels the alert, e.g. when the parent window gets closed,
    /// then this will return a cancel button that was defined in [`AlertOptions`].
    /// If no cancel button was provided then this will be [`AlertButton::CANCEL`].
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    /// [`AlertOptions`]: struct.AlertOptions.html
    /// [`AlertButton::CANCEL`]: struct.AlertButton.html#associatedconstant.CANCEL
    #[inline]
    pub fn button(&self) -> &AlertButton {
        &self.button
    }
}

impl AlertButton {
    /// A positive button with the English label **OK**.
    pub const OK: AlertButton = AlertButton::const_positive("OK");
    /// A cancel button with the English label **Cancel**.
    pub const CANCEL: AlertButton = AlertButton::const_cancel("Cancel");

    /// Create a new positive button with the specified `label`.
    ///
    /// If this is the only button of the alert, use **OK** or a translation of it as the `label`.
    /// Otherwise you should give a short but descriptive label like **Save the draft** that lets
    /// the user know what will happen when they click the button.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    pub fn positive(label: impl Into<String>) -> AlertButton {
        AlertButton {
            button_type: AlertButtonType::Positive,
            label: ConstString::new(label),
        }
    }

    /// Create a new negative button with the specified `label`.
    ///
    /// You should give a short but descriptive label like **Delete the draft** that lets the user
    /// know what will happen when they click the button.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    pub fn negative(label: impl Into<String>) -> AlertButton {
        AlertButton {
            button_type: AlertButtonType::Negative,
            label: ConstString::new(label),
        }
    }

    /// Create a new cancel button with the specified `label`.
    ///
    /// The cancel button `label` should always be **Cancel** or a translation of it.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    pub fn cancel(label: impl Into<String>) -> AlertButton {
        AlertButton {
            button_type: AlertButtonType::Cancel,
            label: ConstString::new(label),
        }
    }

    /// Create a new `const` positive button with the specified `label`.
    ///
    /// This is a specialized `const` version of the [`positive`] function.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`positive`]: #method.positive
    /// [`AlertButton`]: struct.AlertButton.html
    pub const fn const_positive(label: &'static str) -> AlertButton {
        AlertButton {
            button_type: AlertButtonType::Positive,
            label: ConstString::from_static(label),
        }
    }

    /// Create a new `const` negative button with the specified `label`.
    ///
    /// This is a specialized `const` version of the [`negative`] function.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`negative`]: #method.negative
    /// [`AlertButton`]: struct.AlertButton.html
    pub const fn const_negative(label: &'static str) -> AlertButton {
        AlertButton {
            button_type: AlertButtonType::Negative,
            label: ConstString::from_static(label),
        }
    }

    /// Create a new `const` cancel button with the specified `label`.
    ///
    /// This is a specialized `const` version of the [`cancel`] function.
    ///
    /// See [`AlertButton`] for more information.
    ///
    /// [`cancel`]: #method.cancel
    /// [`AlertButton`]: struct.AlertButton.html
    pub const fn const_cancel(label: &'static str) -> AlertButton {
        AlertButton {
            button_type: AlertButtonType::Cancel,
            label: ConstString::from_static(label),
        }
    }
}

impl AlertOptions {
    /// Create a new set of alert options.
    pub fn new() -> AlertOptions {
        AlertOptions {
            buttons: vec![AlertButton::OK],
            ..Default::default()
        }
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

    /// Set the icon of the alert to [`AlertIcon::Information`].
    ///
    /// Use this when the alert dialog is providing important time sensitive information.
    ///
    /// This method is equivalent to calling [`icon`] with [`AlertIcon::Information`].
    ///
    /// [`icon`]: #method.icon
    /// [`AlertIcon::Information`]: enum.AlertIcon.html#variant.Information
    pub fn information(mut self) -> Self {
        self.icon = Some(AlertIcon::Information);
        self
    }

    /// Set the icon of the alert to [`AlertIcon::Warning`].
    ///
    /// Use this when the alert dialog is warning about a potential future problem.
    ///
    /// This method is equivalent to calling [`icon`] with [`AlertIcon::Warning`].
    ///
    /// [`icon`]: #method.icon
    /// [`AlertIcon::Warning`]: enum.AlertIcon.html#variant.Warning
    pub fn warning(mut self) -> Self {
        self.icon = Some(AlertIcon::Warning);
        self
    }

    /// Set the icon of the alert to [`AlertIcon::Error`].
    ///
    /// Use this when the alert dialog is informing of a critical problem that has already happened.
    ///
    /// This method is equivalent to calling [`icon`] with [`AlertIcon::Error`].
    ///
    /// [`icon`]: #method.icon
    /// [`AlertIcon::Error`]: enum.AlertIcon.html#variant.Error
    pub fn error(mut self) -> Self {
        self.icon = Some(AlertIcon::Error);
        self
    }

    /// Set the buttons to be shown on the alert.
    ///
    /// There are three types of buttons - positive, negative, and cancel.
    /// Read [`AlertButton`] for more information about buttons.
    ///
    /// Keep the number of buttons low, almost never showing more than three.
    ///
    /// The order of buttons in this collection matters only in relation to the same button type,
    /// e.g. the order of positive buttons determines their location
    /// only in regards to other positive buttons.
    ///
    /// The order of buttons across button types is determined automatically
    /// based on the guidelines of the specific platform.
    ///
    /// This defaults to a single button - the English [`AlertButton::OK`].
    ///
    /// # Panics
    ///
    /// Panics if the provided `buttons` collection is empty.
    ///
    /// [`AlertButton`]: struct.AlertButton.html
    /// [`AlertButton::OK`]: struct.AlertButton.html#associatedconstant.OK
    pub fn buttons(mut self, buttons: Vec<AlertButton>) -> Self {
        if buttons.is_empty() {
            panic!("Empty alert button collection specified!");
        }
        self.buttons = buttons;
        self
    }
}
