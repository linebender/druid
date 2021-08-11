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

//! macOS backend errors.

//TODO: add a backend error for macOS, based on NSError
use std::any::Any;

use crate::error::ErrorBackend;

#[derive(Debug, Clone)]
pub struct Error;

impl ErrorBackend for Error {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn name(&self) -> String {
        "gtk".into()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "NSError")
    }
}

impl std::error::Error for Error {}
