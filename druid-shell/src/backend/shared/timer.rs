// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::TimerToken;
use std::{cmp::Ordering, time::Instant};

/// A timer is a deadline (`std::Time::Instant`) and a `TimerToken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timer<T> {
    deadline: Instant,
    token: TimerToken,
    pub data: T,
}

impl<T> Timer<T> {
    pub(crate) fn new(deadline: Instant, data: T) -> Self {
        let token = TimerToken::next();
        Self {
            deadline,
            token,
            data,
        }
    }

    pub(crate) fn deadline(&self) -> Instant {
        self.deadline
    }

    pub(crate) fn token(&self) -> TimerToken {
        self.token
    }
}

impl<T: Eq + PartialEq> Ord for Timer<T> {
    /// Ordering is so that earliest deadline sorts first
    // "Earliest deadline first" that a std::collections::BinaryHeap will have the earliest timer
    // at its head, which is just what is needed for timer management.
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline).reverse()
    }
}

impl<T: Eq + PartialEq> PartialOrd for Timer<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
