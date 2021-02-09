// Copyright 2020 The Druid Authors.
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

//! Creating, interpreting, and validating textual representations of values.

use std::str::FromStr;
use std::sync::Arc;

use super::Selection;
use crate::Data;

/// A trait for types that create, interpret, and validate textual representations
/// of values.
///
/// A formatter has two responsiblities: converting a value into an appropriate
/// string representation, and attempting to convert a string back into the
/// appropriate value.
///
/// In addition, a formatter performs validation on *partial* strings; that is,
/// it determines whether or not a string represents a potentially valid value,
/// even if it is not currently valid.
pub trait Formatter<T> {
    /// Return the string representation of this value.
    fn format(&self, value: &T) -> String;
    /// Return the string representation of this value, to be used during editing.
    ///
    /// This can be used if you want the text to differ based on whether or not
    /// it is being edited; for instance you might display a dollar sign when
    /// not editing, but not display it during editing.
    fn format_for_editing(&self, value: &T) -> String {
        self.format(value)
    }

    /// Determine whether the newly edited text is valid for this value type.
    ///
    /// This always returns a [`Validation`] object which indicates if
    /// validation was successful or not, and which can also optionally,
    /// regardless of success or failure, include new text and selection values
    /// that should replace the current ones.
    ///
    ///
    /// # Replacing the text or selection during validation
    ///
    /// Your `Formatter` may wish to change the current text or selection during
    /// editing for a number of reasons. For instance if validation fails, you
    /// may wish to allow editing to continue, but select the invalid region;
    /// alternatively you may consider input valid but want to transform it,
    /// such as by changing case or inserting spaces.
    ///
    /// If you do *not* explicitly set replacement text, and validation is not
    /// successful, the edit will be ignored.
    ///
    /// [`Validation`]: Validation
    fn validate_partial_input(&self, input: &str, sel: &Selection) -> Validation;

    /// The value represented by the input, or an error if the input is invalid.
    ///
    /// This must return `Ok()` for any string created by [`format`].
    ///
    /// [`format`]: #tymethod.format
    fn value(&self, input: &str) -> Result<T, ValidationError>;
}

/// The result of a [`Formatter`] attempting to validate some partial input.
///
/// [`Formatter`]: Formatter
pub struct Validation {
    result: Result<(), ValidationError>,
    /// A manual selection override.
    ///
    /// This will be set as the new selection (regardless of whether or not
    /// validation succeeded or failed)
    pub selection_change: Option<Selection>,
    /// A manual text override.
    ///
    /// This will be set as the new text, regardless of whether or not
    /// validation failed.
    pub text_change: Option<String>,
}

/// An error returned by a [`Formatter`] when it cannot parse input.
///
/// This implements [`source`] so you can access the inner error type.
///
/// [`source`]: std::error::Error::source
// This is currently just a wrapper around the inner error; in the future
// it may grow to contain information such as invalid spans?
// TODO: the fact that this uses `Arc` to work with `Data` is a bit inefficient;
// it means that we will update when a new instance of an identical error occurs.
#[derive(Debug, Clone, Data)]
pub struct ValidationError {
    inner: Arc<dyn std::error::Error>,
}

/// A naive [`Formatter`] for types that implement [`FromStr`].
///
/// For types that implement [`std::fmt::Display`], the [`ParseFormatter::new`]
/// constructor creates a formatter that format's it's value using that trait.
/// If you would like to customize the formatting, you can use the
/// [`ParseFormatter::with_format_fn`] constructor, and pass your own formatting
/// function.
///
/// [`Formatter`]: Formatter
/// [`FromStr`]: std::str::FromStr
#[non_exhaustive]
pub struct ParseFormatter<T> {
    fmt_fn: Box<dyn Fn(&T) -> String>,
}

impl Validation {
    /// Create a `Validation` indicating succes.
    pub fn success() -> Self {
        Validation {
            result: Ok(()),
            selection_change: None,
            text_change: None,
        }
    }

    /// Create a `Validation` with an error indicating the failure reason.
    pub fn failure(err: impl std::error::Error + 'static) -> Self {
        Validation {
            result: Err(ValidationError::new(err)),
            ..Validation::success()
        }
    }

    /// Optionally set a `String` that will replace the current contents.
    pub fn change_text(mut self, text: String) -> Self {
        self.text_change = Some(text);
        self
    }

    /// Optionally set a [`Selection`] that will replace the current one.
    pub fn change_selection(mut self, sel: Selection) -> Self {
        self.selection_change = Some(sel);
        self
    }

    /// Returns `true` if this `Validation` indicates success.
    pub fn is_err(&self) -> bool {
        self.result.is_err()
    }

    /// If validation failed, return the underlying [`ValidationError`].
    ///
    /// [`ValidationError`]: ValidationError
    pub fn error(&self) -> Option<&ValidationError> {
        self.result.as_ref().err()
    }
}

impl ValidationError {
    /// Create a new `ValidationError` with the given error type.
    pub fn new(e: impl std::error::Error + 'static) -> Self {
        ValidationError { inner: Arc::new(e) }
    }
}

impl<T: std::fmt::Display> ParseFormatter<T> {
    /// Create a new `ParseFormatter`.
    pub fn new() -> Self {
        ParseFormatter {
            fmt_fn: Box::new(|val| val.to_string()),
        }
    }
}

impl<T> ParseFormatter<T> {
    /// Create a new `ParseFormatter` using the provided formatting function.
    /// provided function.
    pub fn with_format_fn(f: impl Fn(&T) -> String + 'static) -> Self {
        ParseFormatter {
            fmt_fn: Box::new(f),
        }
    }
}

impl<T> Formatter<T> for ParseFormatter<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + 'static,
{
    fn format(&self, value: &T) -> String {
        (self.fmt_fn)(value)
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        match input.parse::<T>() {
            Ok(_) => Validation::success(),
            Err(e) => Validation::failure(e),
        }
    }

    fn value(&self, input: &str) -> Result<T, ValidationError> {
        input.parse().map_err(ValidationError::new)
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

impl std::error::Error for ValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

impl<T: std::fmt::Display> Default for ParseFormatter<T> {
    fn default() -> Self {
        ParseFormatter::new()
    }
}
