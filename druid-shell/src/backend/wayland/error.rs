// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! wayland platform errors.

use std::{error::Error as StdError, fmt, sync::Arc};
use wayland_client as wl;

#[derive(Debug, Clone)]
pub enum Error {
    /// Error connecting to wayland server.
    Connect(Arc<wl::ConnectError>),
    /// A wayland global either doesn't exist, or doesn't support the version we need.
    Global {
        name: String,
        version: u32,
        inner: Arc<wl::GlobalError>,
    },
    /// An unexpected error occurred. It's not handled by `druid-shell`/wayland, so you should
    /// terminate the app.
    Fatal(Arc<dyn StdError + 'static>),
    String(ErrorString),
    InvalidParent(u32),
    /// general error.
    Err(Arc<dyn StdError + 'static>),
}

impl Error {
    #[allow(clippy::self_named_constructors)]
    pub fn error(e: impl StdError + 'static) -> Self {
        Self::Err(Arc::new(e))
    }

    pub fn fatal(e: impl StdError + 'static) -> Self {
        Self::Fatal(Arc::new(e))
    }

    pub fn global(name: impl Into<String>, version: u32, inner: wl::GlobalError) -> Self {
        Error::Global {
            name: name.into(),
            version,
            inner: Arc::new(inner),
        }
    }

    pub fn string(s: impl Into<String>) -> Self {
        Error::String(ErrorString::from(s))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Connect(e) => write!(f, "could not connect to the wayland server: {e:?}"),
            Self::Global { name, version, .. } => write!(
                f,
                "a required wayland global ({name}@{version}) was unavailable"
            ),
            Self::Fatal(e) => write!(f, "an unhandled error occurred: {e:?}"),
            Self::Err(e) => write!(f, "an unhandled error occurred: {e:?}"),
            Self::String(e) => e.fmt(f),
            Self::InvalidParent(id) => write!(f, "invalid parent window for popup: {id:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Connect(e) => Some(&**e),
            Self::Global { inner, .. } => Some(&**inner),
            Self::Fatal(e) => Some(&**e),
            Self::Err(e) => Some(&**e),
            Self::String(e) => Some(e),
            Self::InvalidParent(_) => None,
        }
    }
}

impl From<wl::ConnectError> for Error {
    fn from(err: wl::ConnectError) -> Self {
        Self::Connect(Arc::new(err))
    }
}

#[derive(Debug, Clone)]
pub struct ErrorString {
    details: String,
}

impl ErrorString {
    pub fn from(s: impl Into<String>) -> Self {
        Self { details: s.into() }
    }
}

impl std::fmt::Display for ErrorString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for ErrorString {
    fn description(&self) -> &str {
        &self.details
    }
}
