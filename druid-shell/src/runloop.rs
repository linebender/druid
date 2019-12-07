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

/// The main application loop.
pub struct RunLoop(platform::RunLoop);

#[derive(PartialEq, Eq)]
pub enum RunFlags {
    /// Run even if druid determines there is an existing instance of the application
    /// already running on the platform.
    MultipleInstances,
}

impl RunLoop {
    /// Create a new `RunLoop`.
    ///
    /// The runloop does not start until [`RunLoop::new`] is called.
    ///
    /// [`RunLoop::new`]: struct.RunLoop.html#method.run
    pub fn new(name: Option<&'static str>, run_flags: Option<RunFlags>) -> RunLoop {
        RunLoop(platform::RunLoop::new(name, run_flags))
    }

    /// Returns true if this `RunLoop`
    /// is a remote connection to an already running `Application`'s runloop.
    pub fn is_remote_connection(&self) -> bool {
        return self.0.is_remote_connection();
    }

    /// Start the runloop.
    ///
    /// This will block the current thread until the program has finished executing.
    pub fn run(&mut self) {
        self.0.run()
    }
}
