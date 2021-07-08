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

//! Timer state.

use std::collections::BTreeSet;
use std::time::Instant;

use crate::window::TimerToken;

pub struct TimerSlots {
    // Note: we can remove this when checked_duration_since lands.
    next_fresh_id: u64,
    free_slots: BTreeSet<u64>,
}

impl TimerSlots {
    pub fn new(starting_ix: u64) -> TimerSlots {
        TimerSlots {
            next_fresh_id: starting_ix,
            free_slots: Default::default(),
        }
    }

    pub fn alloc(&mut self) -> TimerToken {
        if let Some(first) = self.free_slots.iter().next().cloned() {
            self.free_slots.remove(&first);
            TimerToken::from_raw(first)
        } else {
            let result = self.next_fresh_id;
            self.next_fresh_id += 1;
            TimerToken::from_raw(result)
        }
    }

    pub fn free(&mut self, token: TimerToken) {
        let id = token.into_raw();
        if self.next_fresh_id == id + 1 {
            self.next_fresh_id -= 1;
        } else {
            self.free_slots.insert(id);
        }
    }

    /// Compute an elapsed value for SetTimer (in ms)
    pub fn compute_elapsed(&self, deadline: Instant) -> u32 {
        deadline
            .checked_duration_since(Instant::now())
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0)
    }
}
