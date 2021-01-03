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

//! Traits for handling value types.

use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

use crate::kurbo::{self, ParamCurve};
use crate::piet;
use crate::shell::Scale;

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
/// - **`#[data(ignore)]`**
///
/// Skip this field when computing `same`ness.
///
/// If the type you are implementing `Data` on contains some fields that are
/// not relevant to the `Data` impl, you can ignore them with this attribute.
///
/// - **`#[data(same_fn = "path")]`**
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
/// ## Collection types
///
/// `Data` is not implemented for `std` collection types, because comparing them
/// can be expensive. To use collection types with druid, there are two easy options:
/// either wrap the collection in an `Arc`, or build druid with the `im` feature,
/// which adds `Data` implementations to the collections from the [`im` crate],
/// a set of immutable data structures that fit nicely with druid.
///
/// If the `im` feature is used, the `im` crate is reexported from the root
/// of the druid crate.
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
///     #[data(same_fn = "PartialEq::eq")]
///     path: PathBuf,
///     priority: usize,
///     // This field is not part of our data model.
///     #[data(ignore)]
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
/// [`im` crate]: https://docs.rs/im
pub trait Data: Clone + 'static {
    //// ANCHOR: same_fn
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
    //// ANCHOR_END: same_fn
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
impl_data_simple!(i128);
impl_data_simple!(isize);
impl_data_simple!(u8);
impl_data_simple!(u16);
impl_data_simple!(u32);
impl_data_simple!(u64);
impl_data_simple!(u128);
impl_data_simple!(usize);
impl_data_simple!(char);
impl_data_simple!(bool);
impl_data_simple!(std::num::NonZeroI8);
impl_data_simple!(std::num::NonZeroI16);
impl_data_simple!(std::num::NonZeroI32);
impl_data_simple!(std::num::NonZeroI64);
impl_data_simple!(std::num::NonZeroI128);
impl_data_simple!(std::num::NonZeroIsize);
impl_data_simple!(std::num::NonZeroU8);
impl_data_simple!(std::num::NonZeroU16);
impl_data_simple!(std::num::NonZeroU32);
impl_data_simple!(std::num::NonZeroU64);
impl_data_simple!(std::num::NonZeroU128);
impl_data_simple!(std::num::NonZeroUsize);
//TODO: remove me!?
impl_data_simple!(String);

impl Data for &'static str {
    fn same(&self, other: &Self) -> bool {
        ptr::eq(*self, *other)
    }
}

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

impl<T: ?Sized + 'static> Data for Arc<T> {
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(self, other)
    }
}

impl<T: ?Sized + 'static> Data for Rc<T> {
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

impl Data for Scale {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Data for kurbo::Point {
    fn same(&self, other: &Self) -> bool {
        self.x.same(&other.x) && self.y.same(&other.y)
    }
}

impl Data for kurbo::Vec2 {
    fn same(&self, other: &Self) -> bool {
        self.x.same(&other.x) && self.y.same(&other.y)
    }
}

impl Data for kurbo::Size {
    fn same(&self, other: &Self) -> bool {
        self.width.same(&other.width) && self.height.same(&other.height)
    }
}

impl Data for kurbo::Affine {
    fn same(&self, other: &Self) -> bool {
        let rhs = self.as_coeffs();
        let lhs = other.as_coeffs();
        rhs.iter().zip(lhs.iter()).all(|(r, l)| r.same(l))
    }
}

impl Data for kurbo::Insets {
    fn same(&self, other: &Self) -> bool {
        self.x0.same(&other.x0)
            && self.y0.same(&other.y0)
            && self.x1.same(&other.x1)
            && self.y1.same(&other.y1)
    }
}

impl Data for kurbo::Rect {
    fn same(&self, other: &Self) -> bool {
        self.x0.same(&other.x0)
            && self.y0.same(&other.y0)
            && self.x1.same(&other.x1)
            && self.y1.same(&other.y1)
    }
}

impl Data for kurbo::RoundedRect {
    fn same(&self, other: &Self) -> bool {
        self.rect().same(&other.rect()) && self.radius().same(&self.radius())
    }
}

impl Data for kurbo::Arc {
    fn same(&self, other: &Self) -> bool {
        self.center.same(&other.center)
            && self.radii.same(&other.radii)
            && self.start_angle.same(&other.start_angle)
            && self.sweep_angle.same(&other.sweep_angle)
            && self.x_rotation.same(&other.x_rotation)
    }
}

impl Data for kurbo::PathEl {
    fn same(&self, other: &Self) -> bool {
        use kurbo::PathEl::*;
        match (self, other) {
            (MoveTo(p1), MoveTo(p2)) => p1.same(p2),
            (LineTo(p1), LineTo(p2)) => p1.same(p2),
            (QuadTo(x1, y1), QuadTo(x2, y2)) => x1.same(x2) && y1.same(y2),
            (CurveTo(x1, y1, z1), CurveTo(x2, y2, z2)) => x1.same(x2) && y1.same(y2) && z1.same(z2),
            (ClosePath, ClosePath) => true,
            _ => false,
        }
    }
}

impl Data for kurbo::PathSeg {
    fn same(&self, other: &Self) -> bool {
        use kurbo::PathSeg;
        match (self, other) {
            (PathSeg::Line(l1), PathSeg::Line(l2)) => l1.same(l2),
            (PathSeg::Quad(q1), PathSeg::Quad(q2)) => q1.same(q2),
            (PathSeg::Cubic(c1), PathSeg::Cubic(c2)) => c1.same(c2),
            _ => false,
        }
    }
}

impl Data for kurbo::BezPath {
    fn same(&self, other: &Self) -> bool {
        let rhs = self.elements();
        let lhs = other.elements();
        if rhs.len() == lhs.len() {
            rhs.iter().zip(lhs.iter()).all(|(x, y)| x.same(y))
        } else {
            false
        }
    }
}

impl Data for kurbo::Circle {
    fn same(&self, other: &Self) -> bool {
        self.center.same(&other.center) && self.radius.same(&other.radius)
    }
}

impl Data for kurbo::CubicBez {
    fn same(&self, other: &Self) -> bool {
        self.p0.same(&other.p0)
            && self.p1.same(&other.p1)
            && self.p2.same(&other.p2)
            && self.p3.same(&other.p3)
    }
}

impl Data for kurbo::Line {
    fn same(&self, other: &Self) -> bool {
        self.p0.same(&other.p0) && self.p1.same(&other.p1)
    }
}

impl Data for kurbo::ConstPoint {
    fn same(&self, other: &Self) -> bool {
        self.eval(0.).same(&other.eval(0.))
    }
}

impl Data for kurbo::QuadBez {
    fn same(&self, other: &Self) -> bool {
        self.p0.same(&other.p0) && self.p1.same(&other.p1) && self.p2.same(&other.p2)
    }
}

impl Data for piet::Color {
    fn same(&self, other: &Self) -> bool {
        self.as_rgba_u32().same(&other.as_rgba_u32())
    }
}

impl Data for piet::FontFamily {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Data for piet::FontWeight {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Data for piet::FontStyle {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

impl Data for piet::TextAlignment {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

#[cfg(feature = "im")]
impl<T: Data> Data for im::Vector<T> {
    fn same(&self, other: &Self) -> bool {
        // if a vec is small enough that it doesn't require an allocation
        // it is 'inline'; in this case a pointer comparison is meaningless.
        if self.is_inline() {
            self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a.same(b))
        } else {
            self.ptr_eq(other)
        }
    }
}

#[cfg(feature = "im")]
impl<K: Clone + 'static, V: Data> Data for im::HashMap<K, V> {
    fn same(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

#[cfg(feature = "im")]
impl<T: Data> Data for im::HashSet<T> {
    fn same(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

#[cfg(feature = "im")]
impl<K: Clone + 'static, V: Data> Data for im::OrdMap<K, V> {
    fn same(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

#[cfg(feature = "im")]
impl<T: Data> Data for im::OrdSet<T> {
    fn same(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

macro_rules! impl_data_for_array {
    () => {};
    ($this:tt $($rest:tt)*) => {
        impl<T: Data> Data for [T; $this] {
            fn same(&self, other: &Self) -> bool {
                self.iter().zip(other.iter()).all(|(a, b)| a.same(b))
            }
        }
        impl_data_for_array!($($rest)*);
    }
}

impl_data_for_array! { 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 }

#[cfg(test)]
mod test {
    use super::Data;

    #[test]
    fn array_data() {
        let input = [1u8, 0, 0, 1, 0];
        assert!(input.same(&[1u8, 0, 0, 1, 0]));
        assert!(!input.same(&[1u8, 1, 0, 1, 0]));
    }

    #[test]
    #[cfg(feature = "im")]
    fn im_data() {
        for len in 8..256 {
            let input = std::iter::repeat(0_u8).take(len).collect::<im::Vector<_>>();
            let mut inp2 = input.clone();
            assert!(input.same(&inp2));
            inp2.set(len - 1, 98);
            assert!(!input.same(&inp2));
        }
    }

    #[test]
    #[cfg(feature = "im")]
    fn im_vec_different_length() {
        let one = std::iter::repeat(0_u8).take(9).collect::<im::Vector<_>>();
        let two = std::iter::repeat(0_u8).take(10).collect::<im::Vector<_>>();
        assert!(!one.same(&two));
    }

    #[test]
    fn static_strings() {
        let first = "test";
        let same = "test";
        let second = "test2";
        assert!(!Data::same(&first, &second));
        assert!(Data::same(&first, &first));
        // although these are different, the compiler will notice that the string "test" is common,
        // intern it, and reuse it for all "text" `&'static str`s.
        assert!(Data::same(&first, &same));
    }
}
