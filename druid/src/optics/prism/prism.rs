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

pub trait PrismReplacer<A: ?Sized, B: ?Sized>: Prism<A, B> {
    fn replace<'a>(&self, data: &'a mut A, v: B) -> &'a mut A
    where
        B: Sized;

    fn upgrade(&self, v: B) -> A
    where
        B: Sized,
        A: Default + Sized,
    {
        let mut data = A::default();
        self.replace(&mut data, v);
        data
    }
}

pub trait PrismRefReplacer<A: ?Sized, B: ?Sized>: Prism<A, B> {
    fn ref_replace<'a>(&self, data: &'a mut A, v: &B) -> &'a mut A
    where
        B: Clone,
        Self: PrismReplacer<A, B>,
    {
        self.replace(data, v.clone())
    }

    fn ref_upgrade(&self, v: &B) -> A
    where
        B: Clone,
        A: Default + Sized,
        Self: PrismReplacer<A, B>,
    {
        let mut data = A::default();
        self.ref_replace(&mut data, v);
        data
    }
}

impl<A, B, T> PrismRefReplacer<A, B> for T
where
    A: ?Sized,
    B: ?Sized,
    T: PrismReplacer<A, B>,
{
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
        Put: Fn(&mut B, C),
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
            phantom: PhantomData,
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

        #[allow(clippy::unused_unit)]
        match prism.with(data, |newer_data| {
            match prism.with(old_data, |older_data| {
                if !old_data.same(data) {
                    // forwards older and newer data into inner
                    inner.update(ctx, older_data, newer_data, env);
                    // note: the variant wasn't been changed
                    ()
                }
            }) {
                // had both an older and newer data,
                // do nothing more
                Some(()) => (),
                // only had the newer data
                // send newer as both older and newer
                // TODO: check if this is right
                // maybe just ignore the inner update call..
                None => {
                    ctx.request_layout(); // variant was changed
                    inner.update(ctx, newer_data, newer_data, env);
                    ()
                }
            }
        }) {
            // already had the newer data,
            // with or without older data.
            // do nothing more
            Some(()) => (),
            // didn't have the newer data,
            // check if at least the older data is available
            #[allow(clippy::single_match)]
            None => match prism.with(old_data, |older_data| {
                // only had the older data
                // send older as both older and newer
                // TODO: check if this is right
                // maybe just ignore the inner update call..
                ctx.request_layout(); // variant was changed
                inner.update(ctx, older_data, older_data, env);
                ()
            }) {
                // already had only the older data,
                // do nothing more.
                Some(()) => (),
                // didn't have any of the older nor newer data,
                // do nothing.
                // TODO: check if this is right
                None => (),
            },
        }
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

pub struct Variant<Get, GetMut, Replace> {
    get: Get,
    get_mut: GetMut,
    replace: Replace,
}

impl<Get, GetMut, Replace> Variant<Get, GetMut, Replace> {
    pub fn new<T, U>(get: Get, get_mut: GetMut, replace: Replace) -> Self
    where
        T: ?Sized,
        U: Sized,
        Get: Fn(&T) -> Option<&U>,
        GetMut: Fn(&mut T) -> Option<&mut U>,
        Replace: for<'a> Fn(&'a mut T, U) -> &'a mut T,
    {
        Self {
            get,
            get_mut,
            replace,
        }
    }
}

impl<T, U, Get, GetMut, Replace> Prism<T, U> for Variant<Get, GetMut, Replace>
where
    T: ?Sized,
    U: Sized,
    Get: Fn(&T) -> Option<&U>,
    GetMut: Fn(&mut T) -> Option<&mut U>,
    Replace: for<'a> Fn(&'a mut T, U) -> &'a mut T,
{
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> Option<V> {
        (self.get)(data).map(f)
    }

    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> Option<V> {
        (self.get_mut)(data).map(f)
    }
}

impl<T, U, Get, GetMut, Replace> PrismReplacer<T, U> for Variant<Get, GetMut, Replace>
where
    T: ?Sized,
    U: Sized,
    Get: Fn(&T) -> Option<&U>,
    GetMut: Fn(&mut T) -> Option<&mut U>,
    Replace: for<'a> Fn(&'a mut T, U) -> &'a mut T,
{
    fn replace<'a>(&self, data: &'a mut T, v: U) -> &'a mut T
    where
        U: Sized,
    {
        (self.replace)(data, v)
    }
}

#[macro_export]
macro_rules! prism {
    // enum type, variant name
    ($ty:ident, $variant:ident) => {{
        $crate::optics::prism::Variant::new::<$ty, _>(
            // get
            |x: &$ty| {
                if let $ty::$variant(ref v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            // get mut
            |x: &mut $ty| {
                if let $ty::$variant(ref mut v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            // replace
            |x: &mut $ty, v: _| {
                // only works for newtype-like variants
                if let $ty::$variant(ref mut refv) = x {
                    // replace variant's value in-place
                    *refv = v;
                    x
                } else {
                    // upgrade the variant value
                    // and replace the whole enum
                    *x = $ty::$variant(v);
                    x
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

impl<T, U, A, B, C> PrismReplacer<A, C> for Then<T, U, B>
where
    A: ?Sized + Default,
    B: ?Sized + Default,
    C: Sized + Clone,
    T: Prism<A, B> + PrismReplacer<A, B>,
    U: Prism<B, C> + PrismReplacer<B, C>,
{
    /// Given the matching path of `A` -> `B` -> `C`,
    /// it is guaranteed that `A` will end up matching
    /// to `B`, and that `B` will end up match to `C`.
    ///
    /// First it tries replacing `B` -> `C`, and if
    /// it's a success, this means that `A` -> `B` is
    /// already in place.
    ///
    /// Otherwise, if `A` is valued in some  
    /// variant other than `B`, `C` is upgraded
    /// to `B`, and `A` -> `B` is replaced.
    fn replace<'a>(&self, data: &'a mut A, v: C) -> &'a mut A {
        #[allow(clippy::unused_unit)]
        let some_replacement = self.left.with_mut(
            data,
            // A -> B -> C was already set
            // only replaces B -> C
            // (as A -> B is already set)
            |b| {
                self.right.ref_replace(b, &v);
                ()
            },
        );
        if some_replacement.is_none() {
            // couldn't access A -> B,
            // give up the replacement
            // and build B -> C from scratch
            let new_b = self.right.upgrade(v);
            // replace A -> B
            self.left.replace(data, new_b)
        } else {
            // A -> B already set
            // (implicit with/with_mut above)
            data
        }
    }

    fn upgrade<'a>(&self, v: C) -> A
    where
        A: Sized,
    {
        self.left.upgrade(self.right.upgrade(v))
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
        Put: Fn(&mut A, B),
    {
        Self { get, put }
    }
}

impl<A: ?Sized, B, Get, Put> Prism<A, B> for Map<Get, Put>
where
    Get: Fn(&A) -> Option<B>,
    Put: Fn(&mut A, B),
{
    fn with<V, F: FnOnce(&B) -> V>(&self, data: &A, f: F) -> Option<V> {
        (&(self.get)(data)).as_ref().map(f)
    }

    fn with_mut<V, F: FnOnce(&mut B) -> V>(&self, data: &mut A, f: F) -> Option<V> {
        let mut temp = (self.get)(data);
        let x = temp.as_mut().map(f);
        if let Some(b) = temp {
            (self.put)(data, b);
        };
        x
    }
}

impl<A, B, Get, Put> PrismReplacer<A, B> for Map<Get, Put>
where
    Get: Fn(&A) -> Option<B>,
    Put: Fn(&mut A, B),
{
    fn replace<'a>(&self, data: &'a mut A, v: B) -> &'a mut A {
        (self.put)(data, v);
        data
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Deref;

impl<T> Prism<T, T::Target> for Deref
where
    T: ?Sized + ops::Deref + ops::DerefMut,
{
    fn with<V, F: FnOnce(&T::Target) -> V>(&self, data: &T, f: F) -> Option<V> {
        Some(f(data.deref()))
    }

    fn with_mut<V, F: FnOnce(&mut T::Target) -> V>(&self, data: &mut T, f: F) -> Option<V> {
        Some(f(data.deref_mut()))
    }
}

impl<T> PrismReplacer<T, T::Target> for Deref
where
    T: ?Sized + ops::DerefMut,
    T::Target: Sized,
{
    fn replace<'a>(&self, data: &'a mut T, v: T::Target) -> &'a mut T {
        *data.deref_mut() = v;
        data
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

impl<T, I> PrismReplacer<T, T::Output> for Index<I>
where
    T: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
    T::Output: Sized,
{
    fn replace<'a>(&self, data: &'a mut T, v: T::Output) -> &'a mut T {
        data[self.index.clone()] = v;
        data
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

impl<A> PrismReplacer<A, A> for Id {
    fn replace<'a>(&self, data: &'a mut A, v: A) -> &'a mut A {
        *data = v;
        data
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

impl<A, B, L> PrismReplacer<Arc<A>, B> for InArc<L>
where
    A: Clone + Default,
    B: Data,
    L: PrismReplacer<A, B>,
    Arc<A>: ops::DerefMut,
{
    fn replace<'a>(&self, data: &'a mut Arc<A>, v: B) -> &'a mut Arc<A> {
        #[allow(clippy::unused_unit)]
        let some_replacement = self.with_mut(data, |x| {
            *x = v.clone();
            ()
        });
        if some_replacement.is_none() {
            let inner = self.inner.upgrade(v);
            *Arc::make_mut(data) = inner;
        }
        data
    }
}
