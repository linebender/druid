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

//! Timer state.

use std::collections::BTreeSet;
use std::time::Instant;

pub struct TimerSlots {
    // Note: we can remove this when checked_duration_since lands.
    beginning_of_time: Instant,
    sbrk: usize,
    free_slots: BTreeSet<usize>,
}

impl TimerSlots {
    pub fn new(starting_ix: usize) -> TimerSlots {
        TimerSlots {
            beginning_of_time: Instant::now(),
            sbrk: starting_ix,
            free_slots: Default::default(),
        }
    }

    pub fn alloc(&mut self) -> usize {
        if let Some(first) = self.free_slots.iter().next().cloned() {
            self.free_slots.remove(&first);
            first
        } else {
            let result = self.sbrk;
            self.sbrk += 1;
            result
        }
    }

    pub fn free(&mut self, id: usize) {
        if self.sbrk == id + 1 {
            self.sbrk -= 1;
        } else {
            self.free_slots.insert(id);
        }
    }

    /// Compute an elapsed value for SetTimer (in ms)
    pub fn compute_elapsed(&self, deadline: Instant) -> u32 {
        let deadline = deadline.duration_since(self.beginning_of_time);
        let now = self.beginning_of_time.elapsed();
        (deadline.as_micros().saturating_sub(now.as_micros()) / 1000) as u32
    }
}
