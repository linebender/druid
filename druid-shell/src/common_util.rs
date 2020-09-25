// Copyright 2019 The Druid Authors.
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

//! Common functions used by the backends

use std::any::Any;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Strip the access keys from the menu string.
///
/// Changes "E&xit" to "Exit". Actual ampersands are escaped as "&&".
#[cfg(not(feature = "x11"))]
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn strip_access_key(raw_menu_text: &str) -> String {
    let mut saw_ampersand = false;
    let mut result = String::new();
    for c in raw_menu_text.chars() {
        if c == '&' {
            if saw_ampersand {
                result.push(c);
            }
            saw_ampersand = !saw_ampersand;
        } else {
            result.push(c);
            saw_ampersand = false;
        }
    }
    result
}

/// A trait for implementing the boxed callback hack.
pub(crate) trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &dyn Any);
}

impl<F: FnOnce(&dyn Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &dyn Any) {
        (*self)(a)
    }
}

/// An incrementing counter for generating unique ids.
///
/// This can be used safely from multiple threads.
///
/// The counter will overflow if `next()` is called 2^64 - 2 times.
/// If this is possible for your application, and reuse would be undesirable,
/// use something else.
pub struct Counter(AtomicU64);

impl Counter {
    /// Create a new counter.
    pub const fn new() -> Counter {
        Counter(AtomicU64::new(1))
    }

    /// Creates a new counter with a given starting value.
    ///
    /// # Safety
    ///
    /// The value must not be zero.
    pub const unsafe fn new_unchecked(init: u64) -> Counter {
        Counter(AtomicU64::new(init))
    }

    /// Return the next value.
    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }

    /// Return the next value, as a `NonZeroU64`.
    pub fn next_nonzero(&self) -> NonZeroU64 {
        // safe because our initial value is 1 and can only be incremented.
        unsafe { NonZeroU64::new_unchecked(self.0.fetch_add(1, Ordering::Relaxed)) }
    }
}
