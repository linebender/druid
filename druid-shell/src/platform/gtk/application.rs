// Copyright 2019 The xi-editor Authors.
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

use std::cell::RefCell;

use gio::prelude::ApplicationExtManual;
use gio::{ApplicationExt, ApplicationFlags, Cancellable};
use gtk::{Application as GtkApplication, GtkApplicationExt};

use super::clipboard::Clipboard;
use super::util;
use crate::application::AppHandler;

// XXX: The application needs to be global because WindowBuilder::build wants
// to construct an ApplicationWindow, which needs the application, but
// WindowBuilder::build does not get the RunLoop
thread_local!(
    static GTK_APPLICATION: RefCell<Option<GtkApplication>> = RefCell::new(None);
);

pub struct Application;

impl Application {
    pub fn new(_handler: Option<Box<dyn AppHandler>>) -> Application {
        gtk::init().expect("GTK initialization failed");
        util::assert_main_thread();

        // TODO: we should give control over the application ID to the user
        let application = GtkApplication::new(
            Some("com.github.xi-editor.druid"),
            // TODO we set this to avoid connecting to an existing running instance
            // of "com.github.xi-editor.druid" after which we would never receive
            // the "Activate application" below. See pull request druid#384
            // Which shows another way once we have in place a mechanism for
            // communication with remote instances.
            ApplicationFlags::NON_UNIQUE,
        )
        .expect("Unable to create GTK application");

        application.connect_activate(|_app| {
            log::info!("gtk: Activated application");
        });

        application
            .register(None as Option<&Cancellable>)
            .expect("Could not register GTK application");

        GTK_APPLICATION.with(move |x| *x.borrow_mut() = Some(application));
        Application
    }

    pub fn run(&mut self) {
        util::assert_main_thread();

        // TODO: should we pass the command line arguments?
        GTK_APPLICATION.with(|x| {
            x.borrow()
                .as_ref()
                .unwrap() // Safe because we initialized this in RunLoop::new
                .run(&[])
        });
    }

    pub fn quit() {
        util::assert_main_thread();
        with_application(|app| {
            match app.get_active_window() {
                None => {
                    // no application is running, main is not running
                }
                Some(_) => {
                    // we still have an active window, close the run loop
                    app.quit();
                }
            }
        });
    }

    pub fn clipboard() -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}

#[inline]
pub(crate) fn with_application<F, R>(f: F) -> R
where
    F: std::ops::FnOnce(GtkApplication) -> R,
{
    util::assert_main_thread();
    GTK_APPLICATION.with(move |app| {
        let app = app
            .borrow()
            .clone()
            .expect("Tried to manipulate the application before RunLoop::new was called");
        f(app)
    })
}
