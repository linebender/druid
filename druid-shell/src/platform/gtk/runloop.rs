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

//! GTK implementation of runloop.

use std::cell::RefCell;

use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, Cancellable};
use gtk::Application;

use super::util::assert_main_thread;
use crate::runloop::RunFlags;

// XXX: The application needs to be global because WindowBuilder::build wants
// to construct an ApplicationWindow, which needs the application, but
// WindowBuilder::build does not get the RunLoop
thread_local!(
    static GTK_APPLICATION: RefCell<Option<Application>> = RefCell::new(None);
);

/// Container for a GTK runloop
pub struct RunLoop {
    name: String,
}

impl RunLoop {
    pub fn new(name: Option<&'static str>, run_flags: Option<RunFlags>) -> RunLoop {
        assert_main_thread();

        let runloop = RunLoop {
            name: name.map_or("com.github.xi-editor.druid".into(), |s| {
                format!("com.github.xi-editor.druid.{}", s)
            }),
        };

        let gtk_app_flags = run_flags.map_or(ApplicationFlags::FLAGS_NONE, |flag| match flag {
            RunFlags::MultipleInstances => ApplicationFlags::NON_UNIQUE,
        });

        let application = Application::new(Some(&runloop.name), gtk_app_flags)
            .expect("Unable to create GTK application");

        application.connect_activate(|_app| {
            eprintln!("Activated application");
        });

        application
            .register(None as Option<&Cancellable>)
            .expect("Could not register GTK application");

        GTK_APPLICATION.with(move |x| *x.borrow_mut() = Some(application));
        runloop
    }

    pub fn is_remote_connection(&self) -> bool {
        GTK_APPLICATION.with(|x| x.borrow().as_ref().unwrap().get_is_remote())
    }

    pub fn run(&mut self) {
        assert_main_thread();

        // TODO: should we pass the command line arguments?
        GTK_APPLICATION.with(|x| {
            x.borrow()
                .as_ref()
                .unwrap() // Safe because we initialized this in RunLoop::new
                .run(&[])
        });
    }
}

#[inline]
pub(crate) fn with_application<F, R>(f: F) -> R
where
    F: std::ops::FnOnce(Application) -> R,
{
    assert_main_thread();
    GTK_APPLICATION.with(move |app| {
        let app = app
            .borrow()
            .clone()
            .expect("Tried to manipulate the application before RunLoop::new was called");
        f(app)
    })
}
