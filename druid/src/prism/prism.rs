use std::marker::PhantomData;
use std::ops;
use std::sync::Arc;

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::Data;

pub trait Prism<T: ?Sized, U: ?Sized> {
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> Option<V>;
    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> Option<V>;
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

    fn then<Other, C>(self, other: Other) -> Then<Self, Other, B>
    where
        Other: Prism<B, C> + Sized,
        C: ?Sized,
        Self: Sized,
    {
        Then::new(self, other)
    }

    fn map<Get, Put, C>(self, get: Get, put: Put) -> Then<Self, Map<Get, Put>, B>
    where
        Get: Fn(&B) -> Option<C>,
        Put: Fn(&mut B, Option<C>),
        Self: Sized,
    {
        self.then(Map::new(get, put))
    }

    fn deref(self) -> Then<Self, Deref, B>
    where
        B: ops::Deref + ops::DerefMut,
        Self: Sized,
    {
        self.then(Deref)
    }

    fn index<I>(self, index: I) -> Then<Self, Index<I>, B>
    where
        I: Clone,
        B: ops::Index<I> + ops::IndexMut<I>,
        Self: Sized,
    {
        self.then(Index::new(index))
    }

    fn in_arc(self) -> InArc<Self>
    where
        A: Clone,
        B: Data,
        Self: Sized,
    {
        InArc::new(self)
    }
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
        Get: Fn(&T) -> Option<&U>,
        GetMut: Fn(&mut T) -> Option<&mut U>,
    {
        Self { get, get_mut }
    }
}

impl<T, U, Get, GetMut> Prism<T, U> for Variant<Get, GetMut>
where
    T: ?Sized,
    U: ?Sized,
    Get: Fn(&T) -> Option<&U>,
    GetMut: Fn(&mut T) -> Option<&mut U>,
{
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> Option<V> {
        (self.get)(data).map(f)
    }

    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> Option<V> {
        (self.get_mut)(data).map(f)
    }
}

#[macro_export]
macro_rules! prism {
    // enum type, variant name
    ($ty:ident, $variant:ident) => {{
        $crate::prism::Variant::new::<$ty, _>(
            |x| {
                if let $ty::$variant(ref v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            |x| {
                if let $ty::$variant(ref mut v) = x {
                    Some(v)
                } else {
                    None
                }
            },
        )
    }};
}

#[derive(Debug, Copy)]
pub struct Then<T, U, B: ?Sized> {
    left: T,
    right: U,
    _marker: PhantomData<B>,
}

impl<T, U, B: ?Sized> Then<T, U, B> {
    pub fn new<A: ?Sized, C: ?Sized>(left: T, right: U) -> Self
    where
        T: Prism<A, B>,
        U: Prism<B, C>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<T, U, A, B, C> Prism<A, C> for Then<T, U, B>
where
    A: ?Sized,
    B: ?Sized,
    C: ?Sized,
    T: Prism<A, B>,
    U: Prism<B, C>,
{
    fn with<V, F: FnOnce(&C) -> V>(&self, data: &A, f: F) -> Option<V> {
        self.left.with(data, |b| self.right.with(b, f)).flatten()
    }

    fn with_mut<V, F: FnOnce(&mut C) -> V>(&self, data: &mut A, f: F) -> Option<V> {
        self.left
            .with_mut(data, |b| self.right.with_mut(b, f))
            .flatten()
    }
}

impl<T: Clone, U: Clone, B> Clone for Then<T, U, B> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Map<Get, Put> {
    get: Get,
    put: Put,
}

impl<Get, Put> Map<Get, Put> {
    pub fn new<A: ?Sized, B>(get: Get, put: Put) -> Self
    where
        Get: Fn(&A) -> Option<B>,
        Put: Fn(&mut A, Option<B>),
    {
        Self { get, put }
    }
}

impl<A: ?Sized, B, Get, Put> Prism<A, B> for Map<Get, Put>
where
    Get: Fn(&A) -> Option<B>,
    Put: Fn(&mut A, Option<B>),
{
    fn with<V, F: FnOnce(&B) -> V>(&self, data: &A, f: F) -> Option<V> {
        (&(self.get)(data)).as_ref().map(f)
    }

    fn with_mut<V, F: FnOnce(&mut B) -> V>(&self, data: &mut A, f: F) -> Option<V> {
        let mut temp = (self.get)(data);
        let x = temp.as_mut().map(f);
        (self.put)(data, temp);
        x
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Deref;

impl<T: ?Sized> Prism<T, T::Target> for Deref
where
    T: ops::Deref + ops::DerefMut,
{
    fn with<V, F: FnOnce(&T::Target) -> V>(&self, data: &T, f: F) -> Option<V> {
        Some(f(data.deref()))
    }

    fn with_mut<V, F: FnOnce(&mut T::Target) -> V>(&self, data: &mut T, f: F) -> Option<V> {
        Some(f(data.deref_mut()))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Index<I> {
    index: I,
}

impl<I> Index<I> {
    pub fn new(index: I) -> Self {
        Self { index }
    }
}

impl<T, I> Prism<T, T::Output> for Index<I>
where
    T: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
{
    fn with<V, F: FnOnce(&T::Output) -> V>(&self, data: &T, f: F) -> Option<V> {
        Some(f(&data[self.index.clone()]))
    }
    fn with_mut<V, F: FnOnce(&mut T::Output) -> V>(&self, data: &mut T, f: F) -> Option<V> {
        Some(f(&mut data[self.index.clone()]))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Id;

impl<A: ?Sized> Prism<A, A> for Id {
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &A, f: F) -> Option<V> {
        Some(f(data))
    }

    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut A, f: F) -> Option<V> {
        Some(f(data))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InArc<L> {
    inner: L,
}

impl<L> InArc<L> {
    pub fn new<A, B>(inner: L) -> Self
    where
        A: Clone,
        B: Data,
        L: Prism<A, B>,
    {
        Self { inner }
    }
}

impl<A, B, L> Prism<Arc<A>, B> for InArc<L>
where
    A: Clone,
    B: Data,
    L: Prism<A, B>,
{
    fn with<V, F: FnOnce(&B) -> V>(&self, data: &Arc<A>, f: F) -> Option<V> {
        self.inner.with(data, f)
    }

    fn with_mut<V, F: FnOnce(&mut B) -> V>(&self, data: &mut Arc<A>, f: F) -> Option<V> {
        let mut temp = self.inner.with(data, |x| x.clone());
        let v = temp.as_mut().map(f);

        if let Some(true) = self
            .inner
            .with(data, |x| temp.as_ref().map(|b| !x.same(b)))
            .flatten()
        {
            self.inner
                .with_mut(Arc::make_mut(data), |x| temp.map(|b| *x = b));
        }
        v
    }
}
