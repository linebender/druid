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

//! macOS implementation of features at the application scope.

#![allow(non_upper_case_globals)]

use super::clipboard::Clipboard;
use super::util;

use cocoa::appkit::NSApp;
use cocoa::base::{id, nil, YES};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};

pub struct Application;

impl Application {
    pub fn init() {
        unsafe {
            let delegate: id = msg_send![APP_DELEGATE.0, alloc];
            let () = msg_send![delegate, init];
            let () = msg_send![NSApp(), setDelegate: delegate];
        }
    }

    pub fn quit() {
        unsafe {
            let () = msg_send![NSApp(), terminate: nil];
        }
    }

    /// Hide the application this window belongs to. (cmd+H)
    pub fn hide() {
        unsafe {
            let () = msg_send![NSApp(), hide: nil];
        }
    }

    /// Hide all other applications. (cmd+opt+H)
    pub fn hide_others() {
        unsafe {
            let workspace = class!(NSWorkspace);
            let shared: id = msg_send![workspace, sharedWorkspace];
            let () = msg_send![shared, hideOtherApplications];
        }
    }

    pub fn clipboard() -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        unsafe {
            let nslocale_class = class!(NSLocale);
            let locale: id = msg_send![nslocale_class, currentLocale];
            let ident: id = msg_send![locale, localeIdentifier];
            let mut locale = util::from_nsstring(ident);
            if let Some(idx) = locale.chars().position(|c| c == '@') {
                locale.truncate(idx);
            }
            locale
        }
    }
}

struct AppDelegate(*const Class);
unsafe impl Sync for AppDelegate {}

lazy_static! {
    static ref APP_DELEGATE: AppDelegate = unsafe {
        let mut decl = ClassDecl::new("DruidAppDelegate", class!(NSObject))
            .expect("App Delegate definition failed");

        decl.add_method(
            sel!(applicationDidFinishLaunching:),
            application_did_finish_launching as extern "C" fn(&mut Object, Sel, id),
        );
        AppDelegate(decl.register())
    };
}

extern "C" fn application_did_finish_launching(_this: &mut Object, _: Sel, _notification: id) {
    unsafe {
        let () = msg_send![NSApp(), activateIgnoringOtherApps: YES];
    }
}
