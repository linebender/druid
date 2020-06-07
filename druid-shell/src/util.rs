// Copyright 2020 The xi-editor Authors.
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

//! Utility functions for determining the main thread.

use std::fmt::{Debug, Display};
use std::mem;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

/// A convenience wrapper over `String` / `&'static str`.
///
/// This is useful for cases where you want to use a `String` but also
/// want to be able to create the type at compile time via `const` functions.
#[derive(Clone)]
pub struct ConstString(ConstStringValue);

#[derive(Clone)]
enum ConstStringValue {
    Owned(String),
    Static(&'static str),
}

impl ConstString {
    /// Create a new `ConstString` from a `&'static str`.
    pub const fn from_static(value: &'static str) -> ConstString {
        ConstString(ConstStringValue::Static(value))
    }

    /// Create a new `ConstString`.
    pub fn new(value: impl Into<String>) -> ConstString {
        ConstString(ConstStringValue::Owned(value.into()))
    }
}

impl Eq for ConstString {}

impl PartialEq for ConstString {
    fn eq(&self, other: &ConstString) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Deref for ConstString {
    type Target = str;

    fn deref(&self) -> &str {
        match &self.0 {
            ConstStringValue::Owned(s) => s,
            ConstStringValue::Static(s) => *s,
        }
    }
}

impl AsRef<str> for ConstString {
    fn as_ref(&self) -> &str {
        match &self.0 {
            ConstStringValue::Owned(s) => s,
            ConstStringValue::Static(s) => *s,
        }
    }
}

impl Default for ConstString {
    fn default() -> ConstString {
        ConstString::from_static("")
    }
}

impl Display for ConstString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.0 {
            ConstStringValue::Owned(s) => Display::fmt(s, f),
            ConstStringValue::Static(s) => Display::fmt(*s, f),
        }
    }
}

impl Debug for ConstString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.0 {
            ConstStringValue::Owned(s) => Debug::fmt(s, f),
            ConstStringValue::Static(s) => Debug::fmt(*s, f),
        }
    }
}

static MAIN_THREAD_ID: AtomicU64 = AtomicU64::new(0);

#[inline]
fn current_thread_id() -> u64 {
    // TODO: Use .as_u64() instead of mem::transmute
    // when .as_u64() or something similar gets stabilized.
    unsafe { mem::transmute(thread::current().id()) }
}

/// Assert that the current thread is the registered main thread.
///
/// # Panics
///
/// Panics when called from a non-main thread.
pub fn assert_main_thread() {
    let thread_id = current_thread_id();
    let main_thread_id = MAIN_THREAD_ID.load(Ordering::Acquire);
    if thread_id != main_thread_id {
        panic!(
            "Main thread assertion failed {} != {}",
            thread_id, main_thread_id
        );
    }
}

/// Register the current thread as the main thread.
///
/// # Panics
///
/// Panics if the main thread has already been claimed by another thread.
pub fn claim_main_thread() {
    let thread_id = current_thread_id();
    let old_thread_id = MAIN_THREAD_ID.compare_and_swap(0, thread_id, Ordering::AcqRel);
    if old_thread_id != 0 {
        if old_thread_id == thread_id {
            log::warn!("The main thread status was already claimed by the current thread.");
        } else {
            panic!(
                "The main thread status has already been claimed by thread {}",
                thread_id
            );
        }
    }
}

/// Removes the main thread status of the current thread.
///
/// # Panics
///
/// Panics if the main thread status is owned by another thread.
pub fn release_main_thread() {
    let thread_id = current_thread_id();
    let old_thread_id = MAIN_THREAD_ID.compare_and_swap(thread_id, 0, Ordering::AcqRel);
    if old_thread_id == 0 {
        log::warn!("The main thread status was already vacant.");
    } else if old_thread_id != thread_id {
        panic!(
            "The main thread status is owned by another thread {}",
            thread_id
        );
    }
}

/// Wrapper around `RefCell::borrow` that provides error context.
// These are currently only used in the X11 backend, so suppress the unused warning in other
// backends.
#[allow(unused_macros)]
macro_rules! borrow {
    ($val:expr) => {{
        use anyhow::Context;
        $val.try_borrow().with_context(|| {
            format!(
                "[{}:{}] {}",
                std::file!(),
                std::line!(),
                std::stringify!($val)
            )
        })
    }};
}

/// Wrapper around `RefCell::borrow_mut` that provides error context.
#[allow(unused_macros)]
macro_rules! borrow_mut {
    ($val:expr) => {{
        use anyhow::Context;
        $val.try_borrow_mut().with_context(|| {
            format!(
                "[{}:{}] {}",
                std::file!(),
                std::line!(),
                std::stringify!($val)
            )
        })
    }};
}
