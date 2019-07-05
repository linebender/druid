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

/// A trait used to represent value types.
/// 
/// These should be cheap to compare and cheap to clone.
/// 
/// See https://sinusoid.es/lager/model.html#id2 for a well-written
/// explanation of value types (albeit within a C++ context).
pub trait Data: Clone {
    /// Determine whether two values are the same.
    ///
    /// This is intended to always be a fast operation. If it returns
    /// `true`, the two values *must* be equal, but two equal values
    /// need not be considered the same here, as will often be the
    /// case when two copies are separately allocated.
    ///
    /// Note that "equal" above has a slightly different meaning than
    /// `PartialEq`, for example two floating point NaN values should
    /// be considered equal when they have the same bit representation.
    fn same(&self, other: &Self) -> bool;
}

impl Data for u32 {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Data for f64 {
    fn same(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl<T> Data for Arc<T> {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(self, other)
    }
}

// TODO: implementation for other primitive types

// TODO: derive macro
