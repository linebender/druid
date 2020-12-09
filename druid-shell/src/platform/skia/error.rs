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

//! errors.
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Error {
    SkiaError(Arc<skulpin::CreateRendererError>),
    Unimplemented,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::SkiaError(err) => err.fmt(f),
            Error::Unimplemented => write!(f, "Requested an unimplemented feature"),
        }
    }
}

impl std::error::Error for Error {}

impl From<skulpin::CreateRendererError> for Error {
    fn from(err: skulpin::CreateRendererError) -> Error {
        Error::SkiaError(Arc::new(err))
    }
}
