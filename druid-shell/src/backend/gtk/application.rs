// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! GTK implementation of features at the application scope.

use gtk::gio::prelude::ApplicationExtManual;
use gtk::gio::{ApplicationFlags, Cancellable};
use gtk::Application as GtkApplication;

use gtk::prelude::{ApplicationExt, GtkApplicationExt};

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
        let gtk_app = GtkApplication::new(
            Some("com.github.linebender.druid"),
            // TODO we set this to avoid connecting to an existing running instance
            // of "com.github.linebender.druid" after which we would never receive
            // the "Activate application" below. See pull request druid#384
            // Which shows another way once we have in place a mechanism for
            // communication with remote instances.
            ApplicationFlags::NON_UNIQUE,
        );

        gtk_app.connect_activate(|_app| {
            tracing::info!("gtk: Activated application");
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
        self.gtk_app.run();
    }

    pub fn quit(&self) {
        match self.gtk_app.active_window() {
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
        Clipboard {
            selection: gtk::gdk::SELECTION_CLIPBOARD,
        }
    }

    pub fn get_locale() -> String {
        let mut locale: String = gtk::glib::language_names()[0].as_str().into();
        // This is done because the locale parsing library we use expects an unicode locale, but these vars have an ISO locale
        if let Some(idx) = locale.chars().position(|c| c == '.' || c == '@') {
            locale.truncate(idx);
        }
        locale
    }
}

impl crate::platform::linux::ApplicationExt for crate::Application {
    fn primary_clipboard(&self) -> crate::Clipboard {
        crate::Clipboard(Clipboard {
            selection: gtk::gdk::SELECTION_PRIMARY,
        })
    }
}
