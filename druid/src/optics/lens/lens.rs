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

use druid::optics::affine_traversal;

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
pub trait Lens<S: ?Sized, A: ?Sized> {
    /// Get non-mut access to the field.
    ///
    /// Runs the supplied closure with a reference to the data. It's
    /// structured this way, as opposed to simply returning a reference,
    /// so that the data might be synthesized on-the-fly by the lens.
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &S, f: F) -> V;

    /// Get mutable access to the field.
    ///
    /// This method is defined in terms of a closure, rather than simply
    /// yielding a mutable reference, because it is intended to be used
    /// with value-type data (also known as immutable data structures).
    /// For example, a lens for an immutable list might be implemented by
    /// cloning the list, giving the closure mutable access to the clone,
    /// then updating the reference after the closure returns.
    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut S, f: F) -> V;
}

/// Helpers for manipulating `Lens`es
pub trait LensExt<A: ?Sized, B: ?Sized>: Lens<A, B> {
    /// Copy the targeted value out of `data`
    fn get(&self, data: &A) -> B
    where
        B: Clone,
    {
        self.with(data, |x| x.clone())
    }

    /// Set the targeted value in `data` to `value`
    fn put(&self, data: &mut A, value: B)
    where
        B: Sized,
    {
        self.with_mut(data, |x| *x = value);
    }

    /// Combine a `Lens<A, B>` with a function that can transform a `B` and its inverse.
    ///
    /// Useful for cases where the desired value doesn't physically exist in `A`, but can be
    /// computed. For example, a lens like the following might be used to adapt a value with the
    /// range 0-2 for use with a `Widget<f64>` like `Slider` that has a range of 0-1:
    ///
    /// ```
    /// # use druid::*;
    /// let lens = lens!((bool, f64), 1);
    /// assert_eq!(lens.map(|x| x / 2.0, |x, y| *x = y * 2.0).get(&(true, 2.0)), 1.0);
    /// ```
    ///
    /// The computed `C` may represent a whole or only part of the original `B`.
    fn map<Get, Put, C>(self, get: Get, put: Put) -> Then<Self, Map<Get, Put>, B>
    where
        Get: Fn(&B) -> C,
        Put: Fn(&mut B, C),
        Self: Sized,
    {
        affine_traversal::Then::<Map<Get, Put>, A, B, C, _, _>::then(self, Map::new(get, put))
    }

    /// Invoke a type's `Deref` impl
    ///
    /// ```
    /// # use druid::*;
    /// assert_eq!(lens::Id.deref().get(&Box::new(42)), 42);
    /// ```
    fn deref(self) -> Then<Self, Deref, B>
    where
        B: ops::Deref + ops::DerefMut,
        Self: Sized,
    {
        affine_traversal::Then::<Deref, A, B, <B as ops::Deref>::Target, _, _>::then(self, Deref)
    }

    /// Access an index in a container
    ///
    /// ```
    /// # use druid::*;
    /// assert_eq!(lens::Id.index(2).get(&vec![0u32, 1, 2, 3]), 2);
    /// ```
    fn index<I>(self, index: I) -> Then<Self, Index<I>, B>
    where
        I: Clone,
        B: ops::Index<I> + ops::IndexMut<I>,
        Self: Sized,
    {
        affine_traversal::Then::<Index<I>, A, B, <B as ops::Index<I>>::Output, _, _>::then(
            self,
            Index::new(index),
        )
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
        A: Clone,
        B: Data,
        Self: Sized,
    {
        InArc::new(self)
    }
}

impl<A: ?Sized, B: ?Sized, L: Lens<A, B>> LensExt<A, B> for L {}

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

impl<S, A, L, W> Widget<S> for LensWrap<A, L, W>
where
    S: Data,
    A: Data,
    L: Lens<S, A>,
    W: Widget<A>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut S, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with_mut(data, |data| inner.event(ctx, event, data, env))
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &S, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.lifecycle(ctx, event, data, env))
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &S, data: &S, env: &Env) {
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

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &S, env: &Env) -> Size {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.layout(ctx, bc, data, env))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &S, env: &Env) {
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
    pub fn new<S: ?Sized, A: ?Sized>(get: Get, get_mut: GetMut) -> Self
    where
        Get: Fn(&S) -> &A,
        GetMut: Fn(&mut S) -> &mut A,
    {
        Self { get, get_mut }
    }
}

impl<S, A, Get, GetMut> Lens<S, A> for Field<Get, GetMut>
where
    S: ?Sized,
    A: ?Sized,
    Get: Fn(&S) -> &A,
    GetMut: Fn(&mut S) -> &mut A,
{
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &S, f: F) -> V {
        f((self.get)(data))
    }

    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut S, f: F) -> V {
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
pub struct Then<L1, L2, B: ?Sized> {
    left: L1,
    right: L2,
    _marker: PhantomData<B>,
}

impl<L1, L2, B: ?Sized> Then<L1, L2, B> {
    /// Compose two lenses
    ///
    /// See also `LensExt::then`.
    pub fn new<A: ?Sized, C: ?Sized>(left: L1, right: L2) -> Self
    where
        L1: Lens<A, B>,
        L2: Lens<B, C>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<L1, L2, A, B, C> Lens<A, C> for Then<L1, L2, B>
where
    A: ?Sized,
    B: ?Sized,
    C: ?Sized,
    L1: Lens<A, B>,
    L2: Lens<B, C>,
{
    fn with<V, F: FnOnce(&C) -> V>(&self, data: &A, f: F) -> V {
        let bf = |b: &B| self.right.with(b, f);
        self.left.with(data, bf)
    }

    fn with_mut<V, F: FnOnce(&mut C) -> V>(&self, data: &mut A, f: F) -> V {
        self.left.with_mut(data, |b| self.right.with_mut(b, f))
    }
}

impl<L1: Clone, L2: Clone, B> Clone for Then<L1, L2, B> {
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
    pub fn new<A: ?Sized, B>(get: Get, put: Put) -> Self
    where
        Get: Fn(&A) -> B,
        Put: Fn(&mut A, B),
    {
        Self { get, put }
    }
}

impl<A: ?Sized, B, Get, Put> Lens<A, B> for Map<Get, Put>
where
    Get: Fn(&A) -> B,
    Put: Fn(&mut A, B),
{
    fn with<V, F: FnOnce(&B) -> V>(&self, data: &A, f: F) -> V {
        f(&(self.get)(data))
    }

    fn with_mut<V, F: FnOnce(&mut B) -> V>(&self, data: &mut A, f: F) -> V {
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

impl<S: ?Sized> Lens<S, S::Target> for Deref
where
    S: ops::Deref + ops::DerefMut,
{
    fn with<V, F: FnOnce(&S::Target) -> V>(&self, data: &S, f: F) -> V {
        f(data.deref())
    }
    fn with_mut<V, F: FnOnce(&mut S::Target) -> V>(&self, data: &mut S, f: F) -> V {
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

impl<S, I> Lens<S, S::Output> for Index<I>
where
    S: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
{
    fn with<V, F: FnOnce(&S::Output) -> V>(&self, data: &S, f: F) -> V {
        f(&data[self.index.clone()])
    }
    fn with_mut<V, F: FnOnce(&mut S::Output) -> V>(&self, data: &mut S, f: F) -> V {
        f(&mut data[self.index.clone()])
    }
}

/// The identity lens: the lens which does nothing, i.e. exposes exactly the original value.
///
/// Useful for starting a lens combinator chain, or passing to lens-based interfaces.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Id;

impl<S: ?Sized> Lens<S, S> for Id {
    fn with<V, F: FnOnce(&S) -> V>(&self, data: &S, f: F) -> V {
        f(data)
    }

    fn with_mut<V, F: FnOnce(&mut S) -> V>(&self, data: &mut S, f: F) -> V {
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
    pub fn new<S, A>(inner: L) -> Self
    where
        S: Clone,
        A: Data,
        L: Lens<S, A>,
    {
        Self { inner }
    }
}

impl<S, A, L> Lens<Arc<S>, A> for InArc<L>
where
    S: Clone,
    A: Data,
    L: Lens<S, A>,
{
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &Arc<S>, f: F) -> V {
        self.inner.with(data, f)
    }

    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut Arc<S>, f: F) -> V {
        let mut temp = self.inner.with(data, |x| x.clone());
        let v = f(&mut temp);
        if self.inner.with(data, |x| !x.same(&temp)) {
            self.inner.with_mut(Arc::make_mut(data), |x| *x = temp);
        }
        v
    }
}
