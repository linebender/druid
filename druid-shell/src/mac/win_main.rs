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
use cocoa::foundation::NSAutoreleasePool;

use crate::util::assert_main_thread;

pub struct RunLoop {
    ns_app: id,
}

impl RunLoop {
    pub fn new() -> RunLoop {
        assert_main_thread();
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let ns_app = NSApp();
            ns_app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
            RunLoop { ns_app }
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
