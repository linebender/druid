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

//! A simple Bloom filter, used to track child widgets.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use fnv::FnvHasher;

const NUM_BITS: u64 = 64;

// the 'offset_basis' for the fnv-1a hash algorithm.
// see http://www.isthe.com/chongo/tech/comp/fnv/index.html#FNV-param
//
// The first of these is the one described in the algorithm, the second is random.
const OFFSET_ONE: u64 = 0xcbf2_9ce4_8422_2325;
const OFFSET_TWO: u64 = 0xe10_3ad8_2dad_8028;

/// A very simple Bloom filter optimized for small values.
#[derive(Clone, Copy)]
pub(crate) struct Bloom<T: ?Sized> {
    bits: u64,
    data: PhantomData<T>,
    entry_count: usize,
}

impl<T: ?Sized + Hash> Bloom<T> {
    /// Create a new filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of items that have been added to the filter.
    ///
    /// Does not count unique entries; this is just the number of times
    /// `add()` was called since the filter was created or last `clear()`ed.
    // it feels wrong to call this 'len'?
    #[cfg(test)]
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Return the raw bits of this filter.
    #[allow(dead_code)]
    pub fn to_raw(&self) -> u64 {
        self.bits
    }

    /// Remove all entries from the filter.
    pub fn clear(&mut self) {
        self.bits = 0;
        self.entry_count = 0;
    }

    /// Add an item to the filter.
    pub fn add(&mut self, item: &T) {
        let mask = self.make_bit_mask(item);
        self.bits |= mask;
        self.entry_count += 1;
    }

    /// Returns `true` if the item may have been added to the filter.
    ///
    /// This can return false positives, but never false negatives.
    /// Thus `true` means that the item may have been added - or not,
    /// while `false` means that the item has definitely not been added.
    pub fn may_contain(&self, item: &T) -> bool {
        let mask = self.make_bit_mask(item);
        self.bits & mask == mask
    }

    /// Create a new `Bloom` with the items from both filters.
    pub fn union(&self, other: Bloom<T>) -> Bloom<T> {
        Bloom {
            bits: self.bits | other.bits,
            data: PhantomData,
            entry_count: self.entry_count + other.entry_count,
        }
    }

    #[inline]
    fn make_bit_mask(&self, item: &T) -> u64 {
        //NOTE: we use two hash functions, which performs better than a single hash
        // with smaller numbers of items, but poorer with more items. Threshold
        // (given 64 bits) is ~30 items.
        // The reasoning is that with large numbers of items we're already in bad shape;
        // optimize for fewer false positives as we get closer to the leaves.
        // This can be tweaked after profiling.
        let hash1 = self.make_hash(item, OFFSET_ONE);
        let hash2 = self.make_hash(item, OFFSET_TWO);
        (1 << (hash1 % NUM_BITS)) | (1 << (hash2 % NUM_BITS))
    }

    #[inline]
    fn make_hash(&self, item: &T, seed: u64) -> u64 {
        let mut hasher = FnvHasher::with_key(seed);
        item.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: ?Sized> std::fmt::Debug for Bloom<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Bloom: {:064b}: ({})", self.bits, self.entry_count)
    }
}

impl<T: ?Sized> Default for Bloom<T> {
    fn default() -> Self {
        Bloom {
            bits: 0,
            data: PhantomData,
            entry_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn very_good_test() {
        let mut bloom = Bloom::default();
        for i in 0..100 {
            bloom.add(&i);
            assert!(bloom.may_contain(&i));
        }
        bloom.clear();
        for i in 0..100 {
            assert!(!bloom.may_contain(&i));
        }
    }

    #[test]
    fn union() {
        let mut bloom1 = Bloom::default();
        bloom1.add(&0);
        bloom1.add(&1);
        assert!(!bloom1.may_contain(&2));
        assert!(!bloom1.may_contain(&3));
        let mut bloom2 = Bloom::default();
        bloom2.add(&2);
        bloom2.add(&3);
        assert!(!bloom2.may_contain(&0));
        assert!(!bloom2.may_contain(&1));

        let bloom3 = bloom1.union(bloom2);
        assert!(bloom3.may_contain(&0));
        assert!(bloom3.may_contain(&1));
        assert!(bloom3.may_contain(&2));
        assert!(bloom3.may_contain(&3));
    }
}
