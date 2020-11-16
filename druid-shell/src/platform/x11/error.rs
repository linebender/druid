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

//! Errors at the application shell level.

use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Error {
    XError(Arc<x11rb::errors::ReplyError>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let Error::XError(e) = self;
        e.fmt(f)
    }
}

impl std::error::Error for Error {}

impl From<x11rb::protocol::Error> for Error {
    fn from(err: x11rb::protocol::Error) -> Error {
        Error::XError(Arc::new(x11rb::errors::ReplyError::X11Error(err)))
    }
}

impl From<x11rb::errors::ReplyError> for Error {
    fn from(err: x11rb::errors::ReplyError) -> Error {
        Error::XError(Arc::new(err))
    }
}
