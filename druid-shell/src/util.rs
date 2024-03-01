// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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

/// Assert that the current thread is the registered main thread or main thread is not claimed.
///
/// # Panics
///
/// Panics when called from a non-main thread and main thread is claimed.
pub(crate) fn assert_main_thread_or_main_unclaimed() {
    let thread_id = current_thread_id();
    let main_thread_id = MAIN_THREAD_ID.load(Ordering::Acquire);
    if thread_id != main_thread_id && main_thread_id != 0 {
        panic!("Main thread assertion failed {thread_id} != {main_thread_id}");
    }
}

/// Register the current thread as the main thread.
///
/// # Panics
///
/// Panics if the main thread has already been claimed by another thread.
pub(crate) fn claim_main_thread() {
    let thread_id = current_thread_id();
    let old_thread_id =
        MAIN_THREAD_ID.compare_exchange(0, thread_id, Ordering::AcqRel, Ordering::Acquire);
    match old_thread_id {
        Ok(0) => (),
        Ok(_) => unreachable!(), // not possible per the docs
        Err(0) => {
            tracing::warn!("The main thread status was already claimed by the current thread.")
        }
        Err(k) => panic!("The main thread status has already been claimed by thread {k}"),
    }
}

/// Removes the main thread status of the current thread.
///
/// # Panics
///
/// Panics if the main thread status is owned by another thread.
pub(crate) fn release_main_thread() {
    let thread_id = current_thread_id();
    let old_thread_id =
        MAIN_THREAD_ID.compare_exchange(thread_id, 0, Ordering::AcqRel, Ordering::Acquire);
    match old_thread_id {
        Ok(n) if n == thread_id => (),
        Ok(_) => unreachable!(), // not possible per the docs
        Err(0) => tracing::warn!("The main thread status was already vacant."),
        Err(k) => panic!("The main thread status has already been claimed by thread {k}"),
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

/// This is a useful way to clone some values into a `move` closure. Currently only used in the
/// Wayland backend.
#[allow(unused_macros)]
macro_rules! with_cloned {
    ($($val:ident),* ; $($rest:tt)*) => {
        {
            $(
                let $val = $val.clone();
            )*
            $($rest)*
        }
    };
}
