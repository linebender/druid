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

//! Web implementation of features at the application scope.

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::error::Error;

#[derive(Clone)]
pub(crate) struct Application;

impl Application {
    pub fn new() -> Result<Application, Error> {
        Ok(Application)
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {}

    pub fn quit(&self) {}

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}
