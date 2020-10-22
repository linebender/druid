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

use std::marker::PhantomData;
use std::ops;
use std::sync::Arc;

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::Data;

/// A lens is a datatype that gives access to a part of a larger
/// data structure.
///
/// A simple example of a lens is a field of a struct; in this case,
/// the lens itself is zero-sized. Another case is accessing an array
/// element, in which case the lens contains the array index.
///
/// Many `Lens` implementations will be derived by macro, but custom
/// implementations are practical as well.
///
/// The name "lens" is inspired by the [Haskell lens] package, which
/// has generally similar goals. It's likely we'll develop more
/// sophistication, for example combinators to combine lenses.
///
/// [Haskell lens]: http://hackage.haskell.org/package/lens
pub trait Lens<T1: ?Sized, T2: ?Sized> {
    /// Get non-mut access to the field.
    ///
    /// Runs the supplied closure with a reference to the data. It's
    /// structured this way, as opposed to simply returning a reference,
    /// so that the data might be synthesized on-the-fly by the lens.
    fn with<V, F>(&self, data: &T1, f: F) -> V
    where
        F: FnOnce(&T2) -> V;

    /// Get mutable access to the field.
    ///
    /// This method is defined in terms of a closure, rather than simply
    /// yielding a mutable reference, because it is intended to be used
    /// with value-type data (also known as immutable data structures).
    /// For example, a lens for an immutable list might be implemented by
    /// cloning the list, giving the closure mutable access to the clone,
    /// then updating the reference after the closure returns.
    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> V
    where
        F: FnOnce(&mut T2) -> V;
}

/// Helpers for manipulating `Lens`es
pub trait LensExt<T1: ?Sized, T2: ?Sized>: Lens<T1, T2> {
    /// Copy the targeted value out of `data`
    fn get(&self, data: &T1) -> T2
    where
        T2: Clone,
    {
        self.with(data, |x| x.clone())
    }

    /// Set the targeted value in `data` to `value`
    fn put(&self, data: &mut T1, value: T2)
    where
        T2: Sized,
    {
        self.with_mut(data, |x| *x = value);
    }

    /// Combine a `Lens<T1, T2>` with a function that can transform a `T2` and its inverse.
    ///
    /// Useful for cases where the desired value doesn't physically exist in `T1`, but can be
    /// computed. For example, a lens like the following might be used to adapt a value with the
    /// range 0-2 for use with a `Widget<f64>` like `Slider` that has a range of 0-1:
    ///
    /// ```
    /// # use druid::*;
    /// let lens = lens!((bool, f64), 1);
    /// assert_eq!(lens.map(|x| x / 2.0, |x, y| *x = y * 2.0).get(&(true, 2.0)), 1.0);
    /// ```
    ///
    /// The computed `T3` may represent a whole or only part of the original `T2`.
    fn map<Get, Put, T3>(self, get: Get, put: Put) -> Then<Self, Map<Get, Put>, T2>
    where
        Get: Fn(&T2) -> T3,
        Put: Fn(&mut T2, T3),
        Self: Sized,
    {
        Then::new(self, Map::new(get, put))
    }

    /// Invoke a type's `Deref` impl
    ///
    /// ```
    /// # use druid::*;
    /// assert_eq!(lens::Id.deref().get(&Box::new(42)), 42);
    /// ```
    fn deref(self) -> Then<Self, Deref, T2>
    where
        T2: ops::Deref + ops::DerefMut,
        Self: Sized,
    {
        Then::new(self, Deref)
    }

    /// Access an index in a container
    ///
    /// ```
    /// # use druid::*;
    /// assert_eq!(lens::Id.index(2).get(&vec![0u32, 1, 2, 3]), 2);
    /// ```
    fn index<I>(self, index: I) -> Then<Self, Index<I>, T2>
    where
        I: Clone,
        T2: ops::Index<I> + ops::IndexMut<I>,
        Self: Sized,
    {
        Then::new(self, Index::new(index))
    }

    /// Adapt to operate on the contents of an `Arc` with efficient copy-on-write semantics
    ///
    /// ```
    /// # use druid::*; use std::sync::Arc;
    /// let lens = lens::Id.index(2).in_arc();
    /// let mut x = Arc::new(vec![0, 1, 2, 3]);
    /// let original = x.clone();
    /// assert_eq!(lens.get(&x), 2);
    /// lens.put(&mut x, 2);
    /// assert!(Arc::ptr_eq(&original, &x), "no-op writes don't cause a deep copy");
    /// lens.put(&mut x, 42);
    /// assert_eq!(&*x, &[0, 1, 42, 3]);
    /// ```
    fn in_arc(self) -> InArc<Self>
    where
        T1: Clone,
        T2: Data,
        Self: Sized,
    {
        InArc::new(self)
    }
}

impl<S: ?Sized, A: ?Sized, L: Lens<S, A>> LensExt<S, A> for L {}

// A case can be made this should be in the `widget` module.

/// A wrapper for its widget subtree to have access to a part
/// of its parent's data.
///
/// Every widget in druid is instantiated with access to data of some
/// type; the root widget has access to the entire application data.
/// Often, a part of the widget hierarchy is only concerned with a part
/// of that data. The `LensWrap` widget is a way to "focus" the data
/// reference down, for the subtree. One advantage is performance;
/// data changes that don't intersect the scope of the lens aren't
/// propagated.
///
/// Another advantage is generality and reuse. If a widget (or tree of
/// widgets) is designed to work with some chunk of data, then with a
/// lens that same code can easily be reused across all occurrences of
/// that chunk within the application state.
///
/// This wrapper takes a [`Lens`] as an argument, which is a specification
/// of a struct field, or some other way of narrowing the scope.
///
/// [`Lens`]: trait.Lens.html
pub struct LensWrap<U, L, W> {
    inner: W,
    lens: L,
    // The following is a workaround for otherwise getting E0207.
    phantom: PhantomData<U>,
}

impl<U, L, W> LensWrap<U, L, W> {
    /// Wrap a widget with a lens.
    ///
    /// When the lens has type `Lens<T, U>`, the inner widget has data
    /// of type `U`, and the wrapped widget has data of type `T`.
    pub fn new(inner: W, lens: L) -> LensWrap<U, L, W> {
        LensWrap {
            inner,
            lens,
            phantom: Default::default(),
        }
    }
}

impl<T1, T2, L, W> Widget<T1> for LensWrap<T2, L, W>
where
    T1: Data,
    T2: Data,
    L: Lens<T1, T2>,
    W: Widget<T2>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T1, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with_mut(data, |data| inner.event(ctx, event, data, env))
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.lifecycle(ctx, event, data, env))
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T1, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        let lens = &self.lens;
        lens.with(old_data, |old_data| {
            lens.with(data, |data| {
                if !old_data.same(data) {
                    inner.update(ctx, old_data, data, env);
                }
            })
        })
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T1, env: &Env) -> Size {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.layout(ctx, bc, data, env))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T1, env: &Env) {
        let inner = &mut self.inner;
        self.lens.with(data, |data| inner.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}

/// Lens accessing a member of some type using accessor functions
///
/// See also the `lens` macro.
///
/// ```
/// let lens = druid::lens::Field::new(|x: &Vec<u32>| &x[42], |x| &mut x[42]);
/// ```
pub struct Field<Get, GetMut> {
    get: Get,
    get_mut: GetMut,
}

impl<Get, GetMut> Field<Get, GetMut> {
    /// Construct a lens from a pair of getter functions
    pub fn new<T1: ?Sized, T2: ?Sized>(get: Get, get_mut: GetMut) -> Self
    where
        Get: Fn(&T1) -> &T2,
        GetMut: Fn(&mut T1) -> &mut T2,
    {
        Self { get, get_mut }
    }
}

impl<T1, T2, Get, GetMut> Lens<T1, T2> for Field<Get, GetMut>
where
    T1: ?Sized,
    T2: ?Sized,
    Get: Fn(&T1) -> &T2,
    GetMut: Fn(&mut T1) -> &mut T2,
{
    fn with<V, F>(&self, data: &T1, f: F) -> V
    where
        F: FnOnce(&T2) -> V,
    {
        f((self.get)(data))
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> V
    where
        F: FnOnce(&mut T2) -> V,
    {
        f((self.get_mut)(data))
    }
}

/// Construct a lens accessing a type's field
///
/// This is a convenience macro for constructing `Field` lenses for fields or indexable elements.
///
/// ```
/// struct Foo { x: u32 }
/// let lens = druid::lens!(Foo, x);
/// let lens = druid::lens!((u32, bool), 1);
/// let lens = druid::lens!([u8], [4]);
/// ```
#[macro_export]
macro_rules! lens {
    ($ty:ty, [$index:expr]) => {
        $crate::lens::Field::new::<$ty, _>(|x| &x[$index], |x| &mut x[$index])
    };
    ($ty:ty, $field:tt) => {
        $crate::lens::Field::new::<$ty, _>(|x| &x.$field, |x| &mut x.$field)
    };
}

/// `Lens` composed of two lenses joined together
#[derive(Debug, Copy, PartialEq)]
pub struct Then<L1, L2, T2: ?Sized> {
    left: L1,
    right: L2,
    _marker: PhantomData<T2>,
}

impl<L1, L2, T2: ?Sized> Then<L1, L2, T2> {
    /// Compose two lenses
    ///
    /// See also `LensExt::then`.
    pub fn new<T1: ?Sized, T3: ?Sized>(left: L1, right: L2) -> Self
    where
        L1: Lens<T1, T2>,
        L2: Lens<T2, T3>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<L1, L2, T1, T2, T3> Lens<T1, T3> for Then<L1, L2, T2>
where
    T1: ?Sized,
    T2: ?Sized,
    T3: ?Sized,
    L1: Lens<T1, T2>,
    L2: Lens<T2, T3>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> V
    where
        F: FnOnce(&T3) -> V,
    {
        let bf = |b: &T2| self.right.with(b, f);
        self.left.with(data, bf)
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> V
    where
        F: FnOnce(&mut T3) -> V,
    {
        self.left.with_mut(data, |b| self.right.with_mut(b, f))
    }
}

impl<L1: Clone, L2: Clone, T2> Clone for Then<L1, L2, T2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

/// `Lens` built from a getter and a setter
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Map<Get, Put> {
    get: Get,
    put: Put,
}

impl<Get, Put> Map<Get, Put> {
    /// Construct a mapping
    ///
    /// See also `LensExt::map`
    pub fn new<T1: ?Sized, T2>(get: Get, put: Put) -> Self
    where
        Get: Fn(&T1) -> T2,
        Put: Fn(&mut T1, T2),
    {
        Self { get, put }
    }
}

impl<T1: ?Sized, T2, Get, Put> Lens<T1, T2> for Map<Get, Put>
where
    Get: Fn(&T1) -> T2,
    Put: Fn(&mut T1, T2),
{
    fn with<V, F>(&self, data: &T1, f: F) -> V
    where
        F: FnOnce(&T2) -> V,
    {
        f(&(self.get)(data))
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> V
    where
        F: FnOnce(&mut T2) -> V,
    {
        let mut temp = (self.get)(data);
        let x = f(&mut temp);
        (self.put)(data, temp);
        x
    }
}

/// `Lens` for invoking `Deref` and `DerefMut` on a type
///
/// See also `LensExt::deref`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Deref;

impl<T: ?Sized> Lens<T, T::Target> for Deref
where
    T: ops::Deref + ops::DerefMut,
{
    fn with<V, F>(&self, data: &T, f: F) -> V
    where
        F: FnOnce(&T::Target) -> V,
    {
        f(data.deref())
    }
    fn with_mut<V, F>(&self, data: &mut T, f: F) -> V
    where
        F: FnOnce(&mut T::Target) -> V,
    {
        f(data.deref_mut())
    }
}

/// `Lens` for indexing containers
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Index<I> {
    index: I,
}

impl<I> Index<I> {
    /// Construct a lens that accesses a particular index
    ///
    /// See also `LensExt::index`.
    pub fn new(index: I) -> Self {
        Self { index }
    }
}

impl<T, I> Lens<T, T::Output> for Index<I>
where
    T: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
{
    fn with<V, F>(&self, data: &T, f: F) -> V
    where
        F: FnOnce(&T::Output) -> V,
    {
        f(&data[self.index.clone()])
    }
    fn with_mut<V, F>(&self, data: &mut T, f: F) -> V
    where
        F: FnOnce(&mut T::Output) -> V,
    {
        f(&mut data[self.index.clone()])
    }
}

/// The identity lens: the lens which does nothing, i.e. exposes exactly the original value.
///
/// Useful for starting a lens combinator chain, or passing to lens-based interfaces.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Id;

impl<T: ?Sized> Lens<T, T> for Id {
    fn with<V, F>(&self, data: &T, f: F) -> V
    where
        F: FnOnce(&T) -> V,
    {
        f(data)
    }

    fn with_mut<V, F>(&self, data: &mut T, f: F) -> V
    where
        F: FnOnce(&mut T) -> V,
    {
        f(data)
    }
}

/// A `Lens` that exposes data within an `Arc` with copy-on-write semantics
///
/// A copy is only made in the event that a different value is written.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct InArc<L> {
    inner: L,
}

impl<L> InArc<L> {
    /// Adapt a lens to operate on an `Arc`
    ///
    /// See also `LensExt::in_arc`
    pub fn new<T1, T2>(inner: L) -> Self
    where
        T1: Clone,
        T2: Data,
        L: Lens<T1, T2>,
    {
        Self { inner }
    }
}

impl<T1, T2, L> Lens<Arc<T1>, T2> for InArc<L>
where
    T1: Clone,
    T2: Data,
    L: Lens<T1, T2>,
{
    fn with<V, F>(&self, data: &Arc<T1>, f: F) -> V
    where
        F: FnOnce(&T2) -> V,
    {
        self.inner.with(data, f)
    }

    fn with_mut<V, F>(&self, data: &mut Arc<T1>, f: F) -> V
    where
        F: FnOnce(&mut T2) -> V,
    {
        let mut temp = self.inner.with(data, |x| x.clone());
        let v = f(&mut temp);
        if self.inner.with(data, |x| !x.same(&temp)) {
            self.inner.with_mut(Arc::make_mut(data), |x| *x = temp);
        }
        v
    }
}
