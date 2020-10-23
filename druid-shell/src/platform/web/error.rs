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

//! Web platform errors.

use wasm_bindgen::JsValue;

#[derive(Debug, Clone)]
pub enum Error {
    NoWindow,
    NoDocument,
    Js(JsValue),
    JsCast,
    NoElementById(String),
    NoContext,
    Unimplemented,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NoWindow => write!(f, "No global window found"),
            Self::NoDocument => write!(f, "No global document found"),
            Self::Js(err) => write!(f, "JavaScript error: {:?}", err.as_string()),
            Self::JsCast => write!(f, "JavaScript cast error"),
            Self::NoElementById(err) => write!(f, "get_element_by_id error: {}", err),
            Self::NoContext => write!(f, "Failed to get a draw context"),
            Self::Unimplemented => write!(f, "Requested an unimplemented feature"),
        }
    }
}

impl From<JsValue> for Error {
    fn from(js: JsValue) -> Self {
        Self::Js(js)
    }
}

impl std::error::Error for Error {}
