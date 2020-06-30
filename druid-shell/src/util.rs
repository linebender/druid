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

//! Utility functions for determining the main thread.

use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

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
