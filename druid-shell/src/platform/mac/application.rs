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

use std::ffi::c_void;

use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyRegular};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::NSAutoreleasePool;
use lazy_static::lazy_static;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

use super::clipboard::Clipboard;
use super::util;
use crate::application::AppHandler;

static APP_HANDLER_IVAR: &str = "druidAppHandler";

#[derive(Clone)]
pub struct AppState;

pub struct Application {
    ns_app: id,
}

impl AppState {
    pub(crate) fn new() -> AppState {
        AppState
    }
}

impl Application {
    pub fn new(_state: AppState, handler: Option<Box<dyn AppHandler>>) -> Application {
        util::assert_main_thread();
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let delegate: id = msg_send![APP_DELEGATE.0, alloc];
            let () = msg_send![delegate, init];
            let state = DelegateState { handler };
            let handler_ptr = Box::into_raw(Box::new(state));
            (*delegate).set_ivar(APP_HANDLER_IVAR, handler_ptr as *mut c_void);
            let ns_app = NSApp();
            let () = msg_send![ns_app, setDelegate: delegate];
            ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
            Application { ns_app }
        }
    }

    pub fn run(&mut self) {
        unsafe {
            self.ns_app.run();
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

struct DelegateState {
    handler: Option<Box<dyn AppHandler>>,
}

impl DelegateState {
    fn command(&mut self, command: u32) {
        if let Some(inner) = self.handler.as_mut() {
            inner.command(command)
        }
    }
}

struct AppDelegate(*const Class);
unsafe impl Sync for AppDelegate {}

lazy_static! {
    static ref APP_DELEGATE: AppDelegate = unsafe {
        let mut decl = ClassDecl::new("DruidAppDelegate", class!(NSObject))
            .expect("App Delegate definition failed");
        decl.add_ivar::<*mut c_void>(APP_HANDLER_IVAR);

        decl.add_method(
            sel!(applicationDidFinishLaunching:),
            application_did_finish_launching as extern "C" fn(&mut Object, Sel, id),
        );

        decl.add_method(
            sel!(handleMenuItem:),
            handle_menu_item as extern "C" fn(&mut Object, Sel, id),
        );
        AppDelegate(decl.register())
    };
}

extern "C" fn application_did_finish_launching(_this: &mut Object, _: Sel, _notification: id) {
    unsafe {
        let () = msg_send![NSApp(), activateIgnoringOtherApps: YES];
    }
}

/// This handles menu items in the case that all windows are closed.
extern "C" fn handle_menu_item(this: &mut Object, _: Sel, item: id) {
    unsafe {
        let tag: isize = msg_send![item, tag];
        let inner: *mut c_void = *this.get_ivar(APP_HANDLER_IVAR);
        let inner = &mut *(inner as *mut DelegateState);
        (*inner).command(tag as u32);
    }
}
