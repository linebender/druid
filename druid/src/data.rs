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

use std::rc::Rc;
use std::sync::Arc;

pub use druid_derive::Data;

/// A trait used to represent value types.
///
/// These should be cheap to compare and cheap to clone.
///
/// See <https://sinusoid.es/lager/model.html#id2> for a well-written
/// explanation of value types (albeit within a C++ context).
///
/// ## Derive macro
///
/// In general, you can use `derive` to generate a `Data` impl for your types.
///
/// ```
/// # use std::sync::Arc;
/// # use druid::Data;
/// #[derive(Clone, Data)]
/// enum Foo {
///     Case1(i32, f32),
///     Case2 { a: String, b: Arc<i32> }
/// }
/// ```
///
/// ### Derive macro attributes
///
/// There are a number of field attributes available for use with `derive(Data)`.
///
/// - **`#[druid(ignore)]`**
///
/// Skip this field when computing `same`ness.
///
/// If the type you are implementing `Data` on contains some fields that are
/// not relevant to the `Data` impl, you can ignore them with this attribute.
///
/// - **`#[druid(same_fn = "path")]`**
///
/// Use a specific function to compute `same`ness.
///
/// By default, derived implementations of `Data` just call [`Data::same`]
/// recursively on each field. With this attribute, you can specify a
/// custom function that will be used instead.
///
/// This function must have a signature in the form, `fn<T>(&T, &T) -> bool`,
/// where `T` is the type of the field.
///
/// ### Example:
///
/// ```
/// # use std::path::PathBuf;
/// # use std::time::Instant;
/// # use druid::Data;
/// #[derive(Clone, Data)]
/// struct PathEntry {
///     // There's no Data impl for PathBuf, but no problem
///     #[druid(same_fn = "PartialEq::eq")]
///     path: PathBuf,
///     priority: usize,
///     // This field is not part of our data model.
///     #[druid(ignore)]
///     last_read: Instant,
/// }
/// ```
///
/// ## C-style enums
///
/// In the case of a "c-style" enum (one that only contains unit variants,
/// that is where no variant has fields), the implementation that is generated
/// checks for equality. Therefore, such types must also implement `PartialEq`.
///
/// [`Data::same`]: trait.Data.html#tymethod.same
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

/// An impl of `Data` suitable for simple types.
///
/// The `same` method is implemented with equality, so the type should
/// implement `Eq` at least.
macro_rules! impl_data_simple {
    ($t:ty) => {
        impl Data for $t {
            fn same(&self, other: &Self) -> bool {
                self == other
            }
        }
    };
}

impl_data_simple!(i8);
impl_data_simple!(i16);
impl_data_simple!(i32);
impl_data_simple!(i64);
impl_data_simple!(isize);
impl_data_simple!(u8);
impl_data_simple!(u16);
impl_data_simple!(u32);
impl_data_simple!(u64);
impl_data_simple!(usize);
impl_data_simple!(char);
impl_data_simple!(bool);
impl_data_simple!(String);

impl Data for f32 {
    fn same(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl Data for f64 {
    fn same(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl<T: ?Sized> Data for Arc<T> {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(self, other)
    }
}

impl<T: ?Sized> Data for Rc<T> {
    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}

impl<T: Data> Data for Option<T> {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.same(b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: Data> Data for &T {
    fn same(&self, other: &Self) -> bool {
        Data::same(*self, *other)
    }
}

impl<T: Data, U: Data> Data for Result<T, U> {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Ok(a), Ok(b)) => a.same(b),
            (Err(a), Err(b)) => a.same(b),
            _ => false,
        }
    }
}

impl Data for () {
    fn same(&self, _other: &Self) -> bool {
        true
    }
}

impl<T0: Data> Data for (T0,) {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}

impl<T0: Data, T1: Data> Data for (T0, T1) {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0) && self.1.same(&other.1)
    }
}

impl<T0: Data, T1: Data, T2: Data> Data for (T0, T1, T2) {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0) && self.1.same(&other.1) && self.2.same(&other.2)
    }
}

impl<T0: Data, T1: Data, T2: Data, T3: Data> Data for (T0, T1, T2, T3) {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
            && self.1.same(&other.1)
            && self.2.same(&other.2)
            && self.3.same(&other.3)
    }
}

impl<T0: Data, T1: Data, T2: Data, T3: Data, T4: Data> Data for (T0, T1, T2, T3, T4) {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
            && self.1.same(&other.1)
            && self.2.same(&other.2)
            && self.3.same(&other.3)
            && self.4.same(&other.4)
    }
}

impl<T0: Data, T1: Data, T2: Data, T3: Data, T4: Data, T5: Data> Data for (T0, T1, T2, T3, T4, T5) {
    fn same(&self, other: &Self) -> bool {
        self.0.same(&other.0)
            && self.1.same(&other.1)
            && self.2.same(&other.2)
            && self.3.same(&other.3)
            && self.4.same(&other.4)
            && self.5.same(&other.5)
    }
}
