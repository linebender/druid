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

use crate::clipboard::ClipboardItem;

pub struct Application;

impl Application {
    pub fn quit() {
        // Nothing to do: if this is called, we're already shutting down and GTK will pick it up (I hope?)
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
}
