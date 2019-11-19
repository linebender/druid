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

use gtk::GtkApplicationExt;

use super::runloop;
use super::util;
use crate::clipboard::ClipboardItem;

pub struct Application;

impl Application {
    pub fn init() {
        gtk::init().expect("GTK initialization failed");
    }

    pub fn quit() {
        util::assert_main_thread();
        runloop::with_application(|app| {
            match app.get_active_window() {
                None => {
                    // no application is running, main is not running
                }
                Some(_) => {
                    // we still have an active window, close the runLo
                    gtk::main_quit();
                }
            }
        });
    }

    pub fn get_clipboard_contents() -> Option<ClipboardItem> {
        let display = gdk::Display::get_default().unwrap();
        let clipboard = gtk::Clipboard::get_default(&display).unwrap();

        if let Some(gstring) = clipboard.wait_for_text() {
            Some(ClipboardItem::Text(gstring.to_string()))
        } else {
            None
        }
    }

    pub fn set_clipboard_contents(_item: ClipboardItem) {
        log::warn!("set_clipboard_contents is unimplemented on GTK");
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}
