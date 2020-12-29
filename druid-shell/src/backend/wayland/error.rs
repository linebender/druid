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

use std::{fmt, sync::Arc};
use wayland_client as wayland;

#[derive(Debug, Clone)]
pub enum Error {
    /// Error connecting to wayland server.
    // wayland::ConnectError is not `Clone`. TODO do a PR to wayland_client to make it `Clone`.
    Connect(Arc<wayland::ConnectError>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Connect(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<wayland::ConnectError> for Error {
    fn from(err: wayland::ConnectError) -> Self {
        Self::Connect(Arc::new(err))
    }
}
