// Copyright 2018 The xi-editor Authors.
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

#[derive(Debug, Clone)]
pub enum Error {
    // TODO(x11/errors): enumerate `Error`s for X11
    NoError,
}

impl fmt::Display for Error {
    fn fmt(&self, _f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // TODO(x11/errors): implement Error::fmt
        unimplemented!();
    }
}
