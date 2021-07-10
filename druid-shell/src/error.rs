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

//! Errors at the application shell level.

use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::sync::Arc;

pub trait ErrorBackend {
    fn as_any(&self) -> &dyn Any;
    fn name(&self) -> String;
}

/// Shell errors.
#[derive(Debug, Clone)]
pub enum Error {
    /// The Application instance has already been created.
    ApplicationAlreadyExists,
    /// The window has already been destroyed.
    WindowDropped,
    /// Platform specific error.
    Platform(Box<dyn ErrorBackend>),
    /// Other miscellaneous error.
    Other(Arc<anyhow::Error>),
}

impl Debug for dyn ErrorBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.name().as_str() {
            "x11" => std::fmt::Debug::fmt(
                self.as_any()
                    .downcast_ref::<crate::backend::x11::error::Error>()
                    .unwrap(),
                f,
            ),

            x => panic!("cloning unsuported clipboard: {}", x),
        }
    }
}

impl Clone for Box<dyn ErrorBackend> {
    fn clone(&self) -> Self {
        match self.name().as_str() {
            "x11" => Box::new(
                self.as_any()
                    .downcast_ref::<crate::backend::x11::error::Error>()
                    .unwrap()
                    .clone(),
            ),
            x => panic!("cloning unsuported clipboard: {}", x),
        }
    }
}
impl fmt::Display for dyn ErrorBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.name().as_str() {
            "x11" => std::fmt::Display::fmt(
                self.as_any()
                    .downcast_ref::<crate::backend::x11::error::Error>()
                    .unwrap(),
                f,
            ),

            x => panic!("cloning unsuported clipboard: {}", x),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::ApplicationAlreadyExists => {
                write!(f, "An application instance has already been created.")
            }
            Error::Platform(err) => fmt::Display::fmt(err, f),
            Error::WindowDropped => write!(f, "The window has already been destroyed."),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<anyhow::Error> for Error {
    fn from(src: anyhow::Error) -> Error {
        Error::Other(Arc::new(src))
    }
}

#[cfg(feature = "gtk")]
impl From<crate::backend::gtk::error::Error> for Error {
    fn from(src: crate::backend::gtk::error::Error) -> Error {
        Error::Platform(Box::new(src))
    }
}
#[cfg(feature = "x11")]
impl From<crate::backend::x11::error::Error> for Error {
    fn from(src: crate::backend::x11::error::Error) -> Error {
        Error::Platform(Box::new(src))
    }
}
