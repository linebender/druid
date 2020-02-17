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

//! The main application loop.

use crate::platform::runloop as platform;

// TODO: super hacky way to connect the XWindow to the RunLoop. Better way to do it?
#[cfg(all(target_os = "linux", feature = "use_x11"))]
use crate::platform::window::XWindow;

/// The main application loop.
pub struct RunLoop(platform::RunLoop);

impl RunLoop {
    /// Create a new `RunLoop`.
    ///
    /// The runloop does not start until [`RunLoop::new`] is called.
    ///
    /// [`RunLoop::new`]: struct.RunLoop.html#method.run
    pub fn new() -> RunLoop {
        RunLoop(platform::RunLoop::new())
    }

    /// Start the runloop.
    ///
    /// This will block the current thread until the program has finished executing.
    pub fn run(&mut self) {
        self.0.run()
    }

    /// Allow the RunLoop to send events to the correct window.
    // TODO: super hacky way to connect the XWindow to the RunLoop. Better way to do it?
    #[cfg(all(target_os = "linux", feature = "use_x11"))]
    pub(crate) fn add_xwindow(&mut self, x_id: u32, xwindow: XWindow) {
        self.0.add_xwindow(x_id, xwindow);
    }
}
