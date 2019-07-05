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

//! Traits for handling value types.

use std::sync::Arc;

pub trait Data: Clone {
    fn same(&self, other: &Self) -> bool;
}

impl Data for u32 {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T> Data for Arc<T> {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(self, other)
    }
}
