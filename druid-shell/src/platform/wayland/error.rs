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

//! GTK platform errors.

use anyhow::Error as AnyError;
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
    /// An unexpected error occurred. It's not handled by druid-shell/wayland, so you should
    /// terminate the app.
    Fatal(Arc<dyn StdError + 'static>),
}

impl Error {
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Connect(e) => f.write_str("could not connect to the wayland server"),
            Self::Global {
                name,
                version,
                inner,
            } => write!(
                f,
                "a required wayland global ({}@{}) was unavailable",
                name, version
            ),
            Self::Fatal(e) => f.write_str("an unhandled error occurred"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Connect(e) => Some(&**e),
            Self::Global {
                name,
                version,
                inner,
            } => Some(&**inner),
            Self::Fatal(e) => Some(&**e),
        }
    }
}

impl From<wl::ConnectError> for Error {
    fn from(err: wl::ConnectError) -> Self {
        Self::Connect(Arc::new(err))
    }
}
