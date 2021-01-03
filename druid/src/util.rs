// Copyright 2020 The Druid Authors.
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

//! Miscellaneous utility functions.

use std::collections::HashMap;
use std::hash::Hash;
use std::mem;

/// Panic in debug and log::error in release mode.
///
/// This macro is in some way a combination of `panic` and `debug_assert`,
/// but it will log the provided message instead of ignoring it in release builds.
///
/// It's useful when a backtrace would aid debugging but a crash can be avoided in release.
macro_rules! debug_panic {
    () => { ... };
    ($msg:expr) => {
        if cfg!(debug_assertions) {
            panic!($msg);
        } else {
            log::error!($msg);
        }
    };
    ($msg:expr,) => { debug_panic!($msg) };
    ($fmt:expr, $($arg:tt)+) => {
        if cfg!(debug_assertions) {
            panic!($fmt, $($arg)*);
        } else {
            log::error!($fmt, $($arg)*);
        }
    };
}

/// Fast path for equal type extend + drain.
pub trait ExtendDrain {
    /// Extend the collection by draining the entries from `source`.
    ///
    /// This function may swap the underlying memory locations,
    /// so keep that in mind if one of the collections has a large allocation
    /// and it should keep that allocation.
    fn extend_drain(&mut self, source: &mut Self);
}

impl<K, V> ExtendDrain for HashMap<K, V>
where
    K: Eq + Hash + Copy,
    V: Copy,
{
    // Benchmarking this vs just extend+drain with a 10k entry map.
    //
    // running 2 tests
    // test bench_extend       ... bench:       1,971 ns/iter (+/- 566)
    // test bench_extend_drain ... bench:           0 ns/iter (+/- 0)
    fn extend_drain(&mut self, source: &mut Self) {
        if !source.is_empty() {
            if self.is_empty() {
                // If the target is empty we can just swap the pointers.
                mem::swap(self, source);
            } else {
                // Otherwise we need to fall back to regular extend-drain.
                self.extend(source.drain());
            }
        }
    }
}

/// An enum for specifying whether an event was handled.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Handled {
    /// An event was already handled, and shouldn't be propagated to other event handlers.
    Yes,
    /// An event has not yet been handled.
    No,
}

impl Handled {
    /// Has the event been handled yet?
    pub fn is_handled(self) -> bool {
        self == Handled::Yes
    }
}

impl From<bool> for Handled {
    /// Returns `Handled::Yes` if `handled` is true, and `Handled::No` otherwise.
    fn from(handled: bool) -> Handled {
        if handled {
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
