use crate::optics::{affine_traversal, lens};

use std::marker::PhantomData;
use std::ops;
use std::sync::Arc;

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::Data;

pub trait PartialPrism<S: ?Sized, A: ?Sized> {
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &S, f: F) -> Option<V>;
    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut S, f: F) -> Option<V>;
}

pub trait Prism<S: ?Sized, A: ?Sized>: PartialPrism<S, A> {
    fn replace<'a>(&self, data: &'a mut S, v: A) -> &'a mut S
    where
        A: Sized;
}

pub trait DefaultUpgrade<S: ?Sized, A: ?Sized>: PartialPrism<S, A> {
    fn default_upgrade(&self, v: A) -> S
    where
        A: Sized,
        S: Default + Sized,
        Self: Prism<S, A>,
    {
        let mut base = S::default();
        self.replace(&mut base, v);
        base
    }
}

impl<S: ?Sized, A: ?Sized, P> DefaultUpgrade<S, A> for P where P: PartialPrism<S, A> {}

pub trait RefPrism<S: ?Sized, A: ?Sized>: PartialPrism<S, A> {
    fn ref_replace<'a>(&self, data: &'a mut S, v: &A) -> &'a mut S
    where
        A: Clone,
        Self: Prism<S, A>,
    {
        self.replace(data, v.clone())
    }
}

// TODO: see if is necessary
pub trait RefDefaultPrism<S: ?Sized, A: ?Sized>: PartialPrism<S, A> {
    fn ref_default_upgrade(&self, v: &A) -> S
    where
        A: Clone,
        S: Default + Sized,
        Self: Prism<S, A> + RefPrism<S, A>,
    {
        let mut data = S::default();
        self.ref_replace(&mut data, v);
        data
    }
}

pub trait PrismExt<A: ?Sized, B: ?Sized>: PartialPrism<A, B> {
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

    fn and_lens<L, C>(self, lens: L) -> affine_traversal::AndLens<Self, L, B>
    where
        L: lens::Lens<A, C> + Sized,
        C: ?Sized,
        Self: Sized,
    {
        affine_traversal::AndLens::new(self, lens)
    }

    fn after_lens<L, BeforeA>(self, lens: L) -> affine_traversal::ThenAfterLens<L, Self, A>
    where
        L: lens::Lens<BeforeA, A> + Sized,
        BeforeA: ?Sized,
        Self: Sized,
    {
        affine_traversal::ThenAfterLens::new(lens, self)
    }

    fn map<Get, Put, C>(self, get: Get, put: Put) -> Then<Self, Map<Get, Put>, B>
    where
        Get: Fn(&B) -> Option<C>,
        Put: Fn(&mut B, C),
        Self: Sized,
    {
        Then::new(self, Map::new(get, put))
    }

    fn deref(self) -> Then<Self, Deref, B>
    where
        B: ops::Deref + ops::DerefMut,
        Self: Sized,
    {
        Then::new(self, Deref)
    }

    fn index<I>(self, index: I) -> Then<Self, Index<I>, B>
    where
        I: Clone,
        B: ops::Index<I> + ops::IndexMut<I>,
        Self: Sized,
    {
        Then::new(self, Index::new(index))
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

impl<S: ?Sized, A: ?Sized, P: PartialPrism<S, A>> PrismExt<S, A> for P {}

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

impl<S, A, P, W> Widget<S> for PrismWrap<A, P, W>
where
    S: Data,
    A: Data,
    P: PartialPrism<S, A>,
    W: Widget<A>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut S, env: &Env) {
        let inner = &mut self.inner;
        let _opt = self
            .prism
            .with_mut::<(), _>(data, |data| inner.event(ctx, event, data, env));

        // let id = ctx.widget_id();
        // println!("{:?}" id);
        // dbg!("event", _opt);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &S, env: &Env) {
        let id = ctx.widget_id();
        println!("{:?} lifecycle", id);

        let inner = &mut self.inner;
        let _opt = self
            .prism
            .with::<(), _>(data, |data| inner.lifecycle(ctx, event, data, env));

        dbg!((id, _opt));
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &S, data: &S, env: &Env) {
        let id = ctx.widget_id();
        // println!("{:?}" update, id);

        let inner = &mut self.inner;
        let prism = &self.prism;

        #[allow(clippy::blocks_in_if_conditions)]
        match prism.with(data, |newer_data| {
            if prism
                .with(old_data, |older_data| {
                    if !old_data.same(data) {
                        // forwards older and newer data into inner
                        inner.update(ctx, older_data, newer_data, env);
                    }
                })
                .is_none()
            {
                // this is when this variant just got activated
                // ie. does not have an old_data

                // ctx.children_changed();
                // ctx.request_layout(); // variant was changed
                // ctx.request_paint(); // variant was changed
                // inner.update(ctx, newer_data, newer_data, env);
                // inner.update(ctx, newer_data, newer_data, env);
            }
        }) {
            // already had the newer data,
            // with or without older data.
            // do nothing more
            Some(()) => (),
            // didn't have the newer data,
            // check if at least the older data is available
            #[allow(clippy::single_match)]
            None => match prism.with(old_data, |_older_data| {
                // only had the older data
                // send older as both older and newer
                // TODO: check if this is right
                // maybe just ignore the inner update call..
                // ctx.children_changed();
                // ctx.request_layout(); // variant was changed
                // ctx.request_paint(); // variant was changed

                // inner.update(ctx, older_data, older_data, env);
                // inner.update(ctx, older_data, older_data, env);
                dbg!("only has old data");
                ()
            }) {
                // already had only the older data,
                // do nothing more.
                Some(()) => (),
                // didn't have any of the older nor newer data,
                // do nothing.
                // TODO: check if this is right
                None => {
                    // dbg!("no old, no new");
                }
            },
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &S, env: &Env) -> Size {
        let id = ctx.widget_id();
        println!("{:?} layout", id);

        let inner = &mut self.inner;
        self.prism
            .with::<Size, _>(data, |data| inner.layout(ctx, bc, data, env))
            .unwrap_or(Size::ZERO)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &S, env: &Env) {
        let id = ctx.widget_id();
        println!("{:?} paint", id);

        let inner = &mut self.inner;
        let _opt = self.prism.with(data, |data| inner.paint(ctx, data, env));
        // dbg!("paint", _opt);
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
    pub fn new<S, A>(get: Get, get_mut: GetMut, replace: Replace) -> Self
    where
        S: ?Sized,
        A: Sized,
        Get: Fn(&S) -> Option<&A>,
        GetMut: Fn(&mut S) -> Option<&mut A>,
        Replace: for<'a> Fn(&'a mut S, A) -> &'a mut S,
    {
        Self {
            get,
            get_mut,
            replace,
        }
    }
}

impl<S, A, Get, GetMut, Replace> PartialPrism<S, A> for Variant<Get, GetMut, Replace>
where
    S: ?Sized,
    A: Sized,
    Get: Fn(&S) -> Option<&A>,
    GetMut: Fn(&mut S) -> Option<&mut A>,
    Replace: for<'a> Fn(&'a mut S, A) -> &'a mut S,
{
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &S, f: F) -> Option<V> {
        (self.get)(data).map(f)
    }

    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut S, f: F) -> Option<V> {
        (self.get_mut)(data).map(f)
    }
}

impl<S, A, Get, GetMut, Replacer> Prism<S, A> for Variant<Get, GetMut, Replacer>
where
    S: ?Sized,
    A: Sized,
    Get: Fn(&S) -> Option<&A>,
    GetMut: Fn(&mut S) -> Option<&mut A>,
    Replacer: for<'a> Fn(&'a mut S, A) -> &'a mut S,
{
    fn replace<'a>(&self, data: &'a mut S, v: A) -> &'a mut S
    where
        A: Sized,
    {
        (self.replace)(data, v)
    }
}

#[macro_export]
macro_rules! prism {
    // enum type, variant name
    ($ty:ident < $( $N:ident ),* >, $variant:ident) => {{
        $crate::optics::prism::Variant::new::<$ty < $( $N ),* > , _>(
            // get
            |x: &$ty< $( $N ),* >| {
                if let $ty::< $( $N ),* >::$variant(ref v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            // get mut
            |x: &mut $ty< $( $N ),* >| {
                if let $ty::< $( $N ),* >::$variant(ref mut v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            // replace
            |x: &mut $ty< $( $N ),* >, v: _| {
                // only works for newtype-like variants
                if let $ty::< $( $N ),* >::$variant(ref mut refv) = x {
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
    ($ty:ident, $variant:ident) => {{
        $crate::prism!($ty<>, $variant)
    }};
}

#[derive(Debug, Copy, PartialEq)]
pub struct Then<P1, P2, B: ?Sized> {
    left: P1,
    right: P2,
    _marker: PhantomData<B>,
}

impl<P1, P2, B: ?Sized> Then<P1, P2, B> {
    pub fn new<A: ?Sized, C: ?Sized>(left: P1, right: P2) -> Self
    where
        P1: PartialPrism<A, B>,
        P2: PartialPrism<B, C>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<P1, P2, A, B, C> PartialPrism<A, C> for Then<P1, P2, B>
where
    A: ?Sized,
    B: ?Sized,
    C: ?Sized,
    P1: PartialPrism<A, B>,
    P2: PartialPrism<B, C>,
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

impl<P1, P2, A, B, C> Prism<A, C> for Then<P1, P2, B>
where
    A: ?Sized + Default,
    B: ?Sized + Default,
    C: Sized + Clone,
    P1: PartialPrism<A, B> + Prism<A, B> + RefPrism<A, B>,
    P2: PartialPrism<B, C> + Prism<B, C> + RefPrism<B, C>,
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
            let mut new_b = B::default();
            self.right.replace(&mut new_b, v);
            // replace A -> B
            self.left.replace(data, new_b)
        } else {
            // A -> B already set
            // (implicit with/with_mut above)
            data
        }
    }
}

impl<P1: Clone, P2: Clone, B> Clone for Then<P1, P2, B> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

impl<A: ?Sized, B, Get, Put> PartialPrism<A, B> for Map<Get, Put>
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

impl<A, B, Get, Put> Prism<A, B> for Map<Get, Put>
where
    Get: Fn(&A) -> Option<B>,
    Put: Fn(&mut A, B),
{
    fn replace<'a>(&self, data: &'a mut A, v: B) -> &'a mut A {
        (self.put)(data, v);
        data
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Deref;

impl<S> PartialPrism<S, S::Target> for Deref
where
    S: ?Sized + ops::Deref + ops::DerefMut,
{
    fn with<V, F: FnOnce(&S::Target) -> V>(&self, data: &S, f: F) -> Option<V> {
        Some(f(data.deref()))
    }

    fn with_mut<V, F: FnOnce(&mut S::Target) -> V>(&self, data: &mut S, f: F) -> Option<V> {
        Some(f(data.deref_mut()))
    }
}

impl<S> Prism<S, S::Target> for Deref
where
    S: ?Sized + ops::DerefMut,
    S::Target: Sized,
{
    fn replace<'a>(&self, data: &'a mut S, v: S::Target) -> &'a mut S {
        *data.deref_mut() = v;
        data
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Index<I> {
    index: I,
}

impl<I> Index<I> {
    pub fn new(index: I) -> Self {
        Self { index }
    }
}

impl<S, I> PartialPrism<S, S::Output> for Index<I>
where
    S: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
{
    fn with<V, F: FnOnce(&S::Output) -> V>(&self, data: &S, f: F) -> Option<V> {
        Some(f(&data[self.index.clone()]))
    }
    fn with_mut<V, F: FnOnce(&mut S::Output) -> V>(&self, data: &mut S, f: F) -> Option<V> {
        Some(f(&mut data[self.index.clone()]))
    }
}

impl<S, I> Prism<S, S::Output> for Index<I>
where
    S: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
    S::Output: Sized,
{
    fn replace<'a>(&self, data: &'a mut S, v: S::Output) -> &'a mut S {
        data[self.index.clone()] = v;
        data
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Id;

impl<S: ?Sized> PartialPrism<S, S> for Id {
    fn with<V, F: FnOnce(&S) -> V>(&self, data: &S, f: F) -> Option<V> {
        Some(f(data))
    }

    fn with_mut<V, F: FnOnce(&mut S) -> V>(&self, data: &mut S, f: F) -> Option<V> {
        Some(f(data))
    }
}

impl<S> Prism<S, S> for Id {
    fn replace<'a>(&self, data: &'a mut S, v: S) -> &'a mut S {
        *data = v;
        data
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct InArc<P> {
    inner: P,
}

impl<P> InArc<P> {
    pub fn new<S, A>(inner: P) -> Self
    where
        S: Clone,
        A: Data,
        P: PartialPrism<S, A>,
    {
        Self { inner }
    }
}

impl<S, A, P> PartialPrism<Arc<S>, A> for InArc<P>
where
    S: Clone,
    A: Data,
    P: PartialPrism<S, A>,
{
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &Arc<S>, f: F) -> Option<V> {
        self.inner.with(data, f)
    }

    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut Arc<S>, f: F) -> Option<V> {
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

impl<S, A, P> Prism<Arc<S>, A> for InArc<P>
where
    S: Clone + Default,
    A: Data,
    P: Prism<S, A> + DefaultUpgrade<S, A>,
    Arc<S>: ops::DerefMut,
{
    fn replace<'a>(&self, data: &'a mut Arc<S>, v: A) -> &'a mut Arc<S> {
        #[allow(clippy::unused_unit)]
        let some_replacement = self.with_mut(data, |x| {
            *x = v.clone();
            ()
        });
        if some_replacement.is_none() {
            let inner = self.inner.default_upgrade(v);
            *Arc::make_mut(data) = inner;
        }
        data
    }
}
