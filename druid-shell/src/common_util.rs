// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Common functions used by the backends

use std::cell::Cell;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use instant::Instant;

use crate::kurbo::Point;
use crate::WinHandler;

// This is the default timing on windows.
const MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
// the max distance between two clicks for them to count as a multi-click
const MULTI_CLICK_MAX_DISTANCE: f64 = 5.0;

/// Strip the access keys from the menu string.
///
/// Changes "E&xit" to "Exit". Actual ampersands are escaped as "&&".
#[allow(dead_code)]
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
    fn call(self: Box<Self>, a: &mut dyn WinHandler);
}

impl<F: FnOnce(&mut dyn WinHandler) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &mut dyn WinHandler) {
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

/// A small helper for determining the click-count of a mouse-down event.
///
/// Click-count is incremented if both the duration and distance between a pair
/// of clicks are below some threshold.
#[derive(Debug, Clone)]
pub struct ClickCounter {
    max_interval: Cell<Duration>,
    max_distance: Cell<f64>,
    last_click: Cell<Instant>,
    last_pos: Cell<Point>,
    click_count: Cell<u8>,
}

#[allow(dead_code)]
impl ClickCounter {
    /// Create a new ClickCounter with the given interval and distance.
    pub fn new(max_interval: Duration, max_distance: f64) -> ClickCounter {
        ClickCounter {
            max_interval: Cell::new(max_interval),
            max_distance: Cell::new(max_distance),
            last_click: Cell::new(Instant::now()),
            click_count: Cell::new(0),
            last_pos: Cell::new(Point::new(f64::MAX, 0.0)),
        }
    }

    pub fn set_interval_ms(&self, millis: u64) {
        self.max_interval.set(Duration::from_millis(millis))
    }

    pub fn set_distance(&self, distance: f64) {
        self.max_distance.set(distance)
    }

    /// Return the click count for a click occurring now, at the provided position.
    pub fn count_for_click(&self, click_pos: Point) -> u8 {
        let click_time = Instant::now();
        let last_time = self.last_click.replace(click_time);
        let last_pos = self.last_pos.replace(click_pos);
        let elapsed = click_time - last_time;
        let distance = last_pos.distance(click_pos);
        if elapsed > self.max_interval.get() || distance > self.max_distance.get() {
            self.click_count.set(0);
        }
        let click_count = self.click_count.get().saturating_add(1);
        self.click_count.set(click_count);
        click_count
    }
}

impl Default for ClickCounter {
    fn default() -> Self {
        ClickCounter::new(MULTI_CLICK_INTERVAL, MULTI_CLICK_MAX_DISTANCE)
    }
}
