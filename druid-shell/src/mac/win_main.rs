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

//! macOS implementation of runloop.

use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyRegular};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};

use objc::runtime::{Class, Object, Sel};
use objc::declare::ClassDecl;
use crate::util::assert_main_thread;

//const NSCurrentLocaleDidChangeNotification: &str = "NSCurrentLocaleDidChange";
const NSCurrentLocaleDidChangeNotification: &str = "kCFLocaleCurrentLocaleDidChangeNotification";
const NSWindowDidMoveNotification: &str = "NSWindowDidMoveNotification";

pub struct RunLoop {
    ns_app: id,
    delegate: id,
}

impl RunLoop {
    pub fn new() -> RunLoop {
        assert_main_thread();
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let ns_app = NSApp();

            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("DruidAppDelegate", superclass).unwrap();

            extern fn app_will_finish_launching(_: &Object, _: Sel, _notification: id) {
                eprintln!("app will finish launching");
            }

            extern fn locale_did_change(_: &Object, _: Sel, _notification: id) {
                eprintln!("locale did change {:?}", _notification);
            }

            extern fn window_did_move(_: &Object, _: Sel, _notification: id) {
                eprintln!("window did move{:?}", _notification);
            }

            decl.add_method(sel!(applicationWillFinishLaunching:),
            app_will_finish_launching as extern fn(&Object, Sel, id));
            decl.add_method(sel!(localeDidChange:), locale_did_change as extern fn(&Object, Sel, id));
            decl.add_method(sel!(windowDidMove:), window_did_move as extern fn(&Object, Sel, id));
            let delegate_class = decl.register();
            let delegate = msg_send![delegate_class, new];
            msg_send![ns_app, setDelegate: delegate];

            // setup locale change notifications

            let notif_center_class = class!(NSNotificationCenter);
            let notif_center: id = msg_send![notif_center_class, defaultCenter];
            eprintln!("{:?}", notif_center);
            let notif_string = NSString::alloc(nil).init_str(NSCurrentLocaleDidChangeNotification);
            msg_send![notif_center, addObserver:delegate selector:sel!(localeDidChange:) name:notif_string object:nil];
            let notif_string = NSString::alloc(nil).init_str(NSWindowDidMoveNotification);
            msg_send![notif_center, addObserver:delegate selector:sel!(windowDidMove:) name:notif_string object:nil];

            ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
            RunLoop { ns_app, delegate }
        }
    }

    pub fn run(&mut self) {
        unsafe {
            self.ns_app.run();
        }
    }
}

/// Request to quit the application, exiting the runloop.
pub fn request_quit() {
    unsafe {
        // Is this threadsafe, or should we schedule on main loop?
        let () = msg_send![NSApp(), terminate: nil];
    }
}
