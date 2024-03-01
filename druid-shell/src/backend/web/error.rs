// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Web backend errors.

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
            Error::NoWindow => write!(f, "No global window found"),
            Error::NoDocument => write!(f, "No global document found"),
            Error::Js(err) => write!(f, "JavaScript error: {:?}", err.as_string()),
            Error::JsCast => write!(f, "JavaScript cast error"),
            Error::NoElementById(err) => write!(f, "get_element_by_id error: {err}"),
            Error::NoContext => write!(f, "Failed to get a draw context"),
            Error::Unimplemented => write!(f, "Requested an unimplemented feature"),
        }
    }
}

impl From<JsValue> for Error {
    fn from(js: JsValue) -> Error {
        Error::Js(js)
    }
}

impl std::error::Error for Error {}
