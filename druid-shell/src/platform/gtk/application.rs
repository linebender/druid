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

//! GTK implementation of features at the application scope.

use gio::prelude::ApplicationExtManual;
use gio::{ApplicationExt, ApplicationFlags, Cancellable};
use gtk::{Application as GtkApplication, GtkApplicationExt};

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::error::Error;

#[derive(Clone)]
pub(crate) struct Application {
    gtk_app: GtkApplication,
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        // TODO: we should give control over the application ID to the user
        let gtk_app = match GtkApplication::new(
            Some("com.github.linebender.druid"),
            // TODO we set this to avoid connecting to an existing running instance
            // of "com.github.linebender.druid" after which we would never receive
            // the "Activate application" below. See pull request druid#384
            // Which shows another way once we have in place a mechanism for
            // communication with remote instances.
            ApplicationFlags::NON_UNIQUE,
        ) {
            Ok(app) => app,
            Err(err) => return Err(Error::BoolError(err)),
        };

        gtk_app.connect_activate(|_app| {
            log::info!("gtk: Activated application");
        });

        if let Err(err) = gtk_app.register(None as Option<&Cancellable>) {
            return Err(Error::Error(err));
        }

        Ok(Application { gtk_app })
    }

    #[inline]
    pub fn gtk_app(&self) -> &GtkApplication {
        &self.gtk_app
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        // TODO: should we pass the command line arguments?
        self.gtk_app.run(&[]);
    }

    pub fn quit(&self) {
        match self.gtk_app.get_active_window() {
            None => {
                // no application is running, main is not running
            }
            Some(_) => {
                // we still have an active window, close the run loop
                self.gtk_app.quit();
            }
        }
    }

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        glib::get_language_names()[0].as_str().into()
    }
}
