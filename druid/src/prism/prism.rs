use std::marker::PhantomData;
// use std::ops;
// use std::sync::Arc;

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::Data;

pub trait Prism<T: ?Sized, U: ?Sized> {
    fn with_raw<V, F: FnOnce(Option<&U>) -> Option<V>>(&self, data: &T, f: F) -> Option<V>;
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> Option<V> {
        self.with_raw::<V, _>(data, |data| data.map(|data| f(data)))
    }

    fn with_raw_mut<V, F: FnOnce(Option<&mut U>) -> Option<V>>(
        &self,
        data: &mut T,
        f: F,
    ) -> Option<V>;
    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> Option<V> {
        self.with_raw_mut::<V, _>(data, |data| data.map(|data| f(data)))
    }
}

pub trait PrismExt<A: ?Sized, B: ?Sized>: Prism<A, B> {
    /// Copy the targeted value out of `data`
    fn get(&self, data: &A) -> Option<B>
    where
        B: Clone,
    {
        self.with::<B, _>(data, |x| x.clone())
    }

    /// Set the targeted value in `data` to `value`
    fn put(&self, data: &mut A, value: Option<B>)
    where
        B: Sized + Clone,
    {
        self.with_mut::<Option<B>, _>(data, |x| {
            match (x, value) {
                // update the value; no discriminant change
                (x, Some(value)) => {
                    *x = value;
                    None // no problem
                }
                // would need to change into another discriminant
                (x, None) => {
                    // (only haws access to x, not the whole option)
                    Some(x.clone()) // cannot put
                }
            }
        });
    }

    /*
    /// Compose a `Lens<A, B>` with a `Lens<B, C>` to produce a `Lens<A, C>`
    ///
    /// ```
    /// # use druid::*;
    /// struct Foo { x: (u32, bool) }
    /// let lens = lens!(Foo, x).then(lens!((u32, bool), 1));
    /// assert_eq!(lens.get(&Foo { x: (0, true) }), true);
    /// ```
    fn then<Other, C>(self, other: Other) -> Then<Self, Other, B>
    where
        Other: Lens<B, C> + Sized,
        C: ?Sized,
        Self: Sized,
    {
        Then::new(self, other)
    }
    */

    /*
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
        self.then(Map::new(get, put))
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
        self.then(Deref)
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
        self.then(Index::new(index))
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
    */
}

impl<A: ?Sized, B: ?Sized, T: Prism<A, B>> PrismExt<A, B> for T {}

pub struct PrismWrap<U, P, W> {
    inner: W,
    prism: P,
    // The following is a workaround for otherwise getting E0207.
    phantom: PhantomData<U>,
}

impl<U, P, W> PrismWrap<U, P, W> {
    pub fn new(inner: W, prism: P) -> PrismWrap<U, P, W> {
        PrismWrap {
            inner,
            prism,
            phantom: Default::default(),
        }
    }
}

impl<T, U, P, W> Widget<T> for PrismWrap<U, P, W>
where
    T: Data,
    U: Data,
    P: Prism<T, U>,
    W: Widget<U>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self
            .prism
            .with_mut::<(), _>(data, |data| inner.event(ctx, event, data, env));
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self
            .prism
            .with::<(), _>(data, |data| inner.lifecycle(ctx, event, data, env));
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        let inner = &mut self.inner;
        let prism = &self.prism;
        prism.with(old_data, |old_data| {
            prism.with(data, |data| {
                if !old_data.same(data) {
                    inner.update(ctx, old_data, data, env);
                }
            })
        });
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let inner = &mut self.inner;
        self.prism
            .with::<Size, _>(data, |data| inner.layout(ctx, bc, data, env))
            .unwrap_or(Size::ZERO)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let inner = &mut self.inner;
        self.prism.with(data, |data| inner.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}

pub struct Variant<Get, GetMut> {
    get: Get,
    get_mut: GetMut,
}

impl<Get, GetMut> Variant<Get, GetMut> {
    pub fn new<T: ?Sized, U: ?Sized>(get: Get, get_mut: GetMut) -> Self
    where
        Get: Fn(Option<&T>) -> Option<&U>,
        GetMut: Fn(Option<&mut T>) -> Option<&mut U>,
    {
        Self { get, get_mut }
    }
}

impl<T, U, Get, GetMut> Prism<T, U> for Variant<Get, GetMut>
where
    T: ?Sized,
    U: ?Sized,
    Get: Fn(&T) -> &U,
    GetMut: Fn(&mut T) -> &mut U,
{
    fn with_raw<V, F: FnOnce(Option<&U>) -> Option<V>>(&self, data: &T, f: F) -> Option<V> {
        f(Some((self.get)(data)))
    }

    fn with_raw_mut<V, F: FnOnce(Option<&mut U>) -> Option<V>>(
        &self,
        data: &mut T,
        f: F,
    ) -> Option<V> {
        f(Some((self.get_mut)(data)))
    }
}
