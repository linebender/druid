#![allow(missing_docs)]

use crate::optics::{affine_traversal as aff, lens};
use crate::Data;
use std::marker::PhantomData;
use std::ops;
use std::sync::Arc;

pub trait PartialPrism<T1: ?Sized, T2: ?Sized> {
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T2) -> V;
    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T2) -> V;
}

pub trait Prism<T1: ?Sized, T2: ?Sized>: PartialPrism<T1, T2> {
    fn replace<'a>(&self, data: &'a mut T1, v: T2) -> &'a mut T1
    where
        T2: Sized;
}

pub trait DefaultUpgrade<T1: ?Sized, T2: ?Sized>: PartialPrism<T1, T2> {
    fn default_upgrade(&self, v: T2) -> T1
    where
        T1: Default + Sized,
        T2: Sized,
        Self: Prism<T1, T2>,
    {
        let mut base = T1::default();
        self.replace(&mut base, v);
        base
    }
}

impl<T1: ?Sized, T2: ?Sized, P> DefaultUpgrade<T1, T2> for P where P: PartialPrism<T1, T2> {}

pub trait RefPrism<T1: ?Sized, T2: ?Sized>: PartialPrism<T1, T2> {
    fn ref_replace<'a>(&self, data: &'a mut T1, v: &T2) -> &'a mut T1
    where
        T2: Clone,
        Self: Prism<T1, T2>,
    {
        self.replace(data, v.clone())
    }
}

// TODO: see if is necessary
pub trait RefDefaultPrism<T1: ?Sized, T2: ?Sized>: PartialPrism<T1, T2> {
    fn ref_default_upgrade(&self, v: &T2) -> T1
    where
        T1: Default + Sized,
        T2: Clone,
        Self: Prism<T1, T2> + RefPrism<T1, T2>,
    {
        let mut data = T1::default();
        self.ref_replace(&mut data, v);
        data
    }
}

pub trait PrismExt<T1: ?Sized, T2: ?Sized>: PartialPrism<T1, T2> {
    /// Copy the targeted value out of `data`
    fn get(&self, data: &T1) -> Option<T2>
    where
        T2: Clone,
    {
        self.with::<T2, _>(data, |x| x.clone())
    }

    /// Set the targeted value in `data` to `value`
    fn put(&self, data: &mut T1, value: Option<T2>)
    where
        T2: Sized + Clone,
    {
        self.with_mut::<Option<T2>, _>(data, |x| {
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

    fn after_lens<L, T0>(self, lens: L) -> aff::ThenAfterLens<L, Self, T1>
    where
        T0: ?Sized,
        Self: Sized,
        L: lens::Lens<T0, T1> + Sized,
    {
        aff::ThenAfterLens::new(lens, self)
    }

    fn map<Get, Put, T3>(self, get: Get, put: Put) -> Then<Self, Map<Get, Put>, T2>
    where
        Get: Fn(&T2) -> Option<T3>,
        Put: Fn(&mut T2, T3),
        Self: Sized,
    {
        Then::new(self, Map::new(get, put))
    }

    fn deref(self) -> Then<Self, Deref, T2>
    where
        T2: ops::Deref + ops::DerefMut,
        Self: Sized,
    {
        Then::new(self, Deref)
    }

    fn index<I>(self, index: I) -> Then<Self, Index<I>, T2>
    where
        I: Clone,
        T2: ops::Index<I> + ops::IndexMut<I>,
        Self: Sized,
    {
        Then::new(self, Index::new(index))
    }

    fn in_arc(self) -> InArc<Self>
    where
        T1: Clone,
        T2: Data,
        Self: Sized,
    {
        InArc::new(self)
    }

    fn discard(self) -> aff::ThenLens<Self, lens::Unit<T2>, T2>
    where
        Self: Sized,
        T2: Sized,
    {
        use aff::Then;
        self.then(lens::Unit::default())
    }
}

impl<T1: ?Sized, T2: ?Sized, P: PartialPrism<T1, T2>> PrismExt<T1, T2> for P {}

pub struct Variant<Get, GetMut, Replace> {
    get: Get,
    get_mut: GetMut,
    replace: Replace,
}

impl<Get, GetMut, Replace> Variant<Get, GetMut, Replace> {
    pub fn new<T1, T2>(get: Get, get_mut: GetMut, replace: Replace) -> Self
    where
        T1: ?Sized,
        T2: Sized,
        Get: Fn(&T1) -> Option<&T2>,
        GetMut: Fn(&mut T1) -> Option<&mut T2>,
        Replace: for<'a> Fn(&'a mut T1, T2) -> &'a mut T1,
    {
        Self {
            get,
            get_mut,
            replace,
        }
    }
}

impl<T1, T2, Get, GetMut, Replace> PartialPrism<T1, T2> for Variant<Get, GetMut, Replace>
where
    T1: ?Sized,
    T2: Sized,
    Get: Fn(&T1) -> Option<&T2>,
    GetMut: Fn(&mut T1) -> Option<&mut T2>,
    Replace: for<'a> Fn(&'a mut T1, T2) -> &'a mut T1,
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T2) -> V,
    {
        (self.get)(data).map(f)
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T2) -> V,
    {
        (self.get_mut)(data).map(f)
    }
}

impl<T1, T2, Get, GetMut, Replacer> Prism<T1, T2> for Variant<Get, GetMut, Replacer>
where
    T1: ?Sized,
    T2: Sized,
    Get: Fn(&T1) -> Option<&T2>,
    GetMut: Fn(&mut T1) -> Option<&mut T2>,
    Replacer: for<'a> Fn(&'a mut T1, T2) -> &'a mut T1,
{
    fn replace<'a>(&self, data: &'a mut T1, v: T2) -> &'a mut T1
    where
        T2: Sized,
    {
        (self.replace)(data, v)
    }
}

/// Construct a prism accessing a type's variant
///
/// This is a convenience macro for constructing `Variant` prisms for enums.
///
/// ```
/// # use crate::druid::PrismExt;
/// let res_ok = druid::prism!(Result<bool, u8>, Ok);
/// assert_eq!(Some(true), res_ok.get(&Ok(true)));
///
/// # use druid::optics::affine_traversal::Then;
/// type Outer = Result<Inner, f32>;
/// type Inner = (u32, bool);
/// let ok1 = druid::prism!(Outer, Ok).then(druid::lens!(Inner, 1));
/// assert_eq!(Some(true), ok1.get(&Outer::Ok((3, true))));
/// assert_eq!(None, ok1.get(&Outer::Err(5.5)));
/// ```
#[macro_export]
macro_rules! prism {
    // enum type, variant name
    ($ty:ident < $( $N:ident ),* >, $variant:ident) => {{
        $crate::optics::prism::Variant::new::<$ty < $( $N ),* > , _>(
            // get
            move |x: &$ty< $( $N ),* >| {
                if let $ty::< $( $N ),* >::$variant(ref v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            // get mut
            move |x: &mut $ty< $( $N ),* >| {
                if let $ty::< $( $N ),* >::$variant(ref mut v) = x {
                    Some(v)
                } else {
                    None
                }
            },
            // replace
            move |x: &mut $ty< $( $N ),* >, v: _| {
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
pub struct Then<P1, P2, T2: ?Sized> {
    left: P1,
    right: P2,
    _marker: PhantomData<T2>,
}

impl<P1, P2, T2: ?Sized> Then<P1, P2, T2> {
    pub fn new<T1: ?Sized, T3: ?Sized>(left: P1, right: P2) -> Self
    where
        P1: PartialPrism<T1, T2>,
        P2: PartialPrism<T2, T3>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<P1, P2, T1, T2, T3> PartialPrism<T1, T3> for Then<P1, P2, T2>
where
    T1: ?Sized,
    T2: ?Sized,
    T3: ?Sized,
    P1: PartialPrism<T1, T2>,
    P2: PartialPrism<T2, T3>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T3) -> V,
    {
        self.left.with(data, |b| self.right.with(b, f)).flatten()
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T3) -> V,
    {
        self.left
            .with_mut(data, |b| self.right.with_mut(b, f))
            .flatten()
    }
}

impl<P1, P2, T1, T2, T3> Prism<T1, T3> for Then<P1, P2, T2>
where
    T1: ?Sized + Default,
    T2: ?Sized + Default,
    T3: Sized + Clone,
    P1: PartialPrism<T1, T2> + Prism<T1, T2> + RefPrism<T1, T2>,
    P2: PartialPrism<T2, T3> + Prism<T2, T3> + RefPrism<T2, T3>,
{
    /// Given the matching path of `T1` -> `T2` -> `T3`,
    /// it is guaranteed that `T1` will end up matching
    /// to `T2`, and that `T2` will end up match to `T3`.
    ///
    /// First it tries replacing `T2` -> `T3`, and if
    /// it's a success, this means that `T1` -> `T2` is
    /// already in place.
    ///
    /// Otherwise, if `T1` is valued in some  
    /// variant other than `T2`, `T3` is upgraded
    /// to `T2`, and `T1` -> `T2` is replaced.
    fn replace<'a>(&self, data: &'a mut T1, v: T3) -> &'a mut T1 {
        #[allow(clippy::unused_unit)]
        let some_replacement = self.left.with_mut(
            data,
            // T1 -> T2 -> T3 was already set
            // only replaces T2 -> T3
            // (as T1 -> T2 is already set)
            |b| {
                self.right.ref_replace(b, &v);
                ()
            },
        );
        if some_replacement.is_none() {
            // couldn't access T1 -> T2,
            // give up the replacement
            // and build T2 -> T3 from scratch
            let mut new_b = T2::default();
            self.right.replace(&mut new_b, v);
            // replace T1 -> T2
            self.left.replace(data, new_b)
        } else {
            // T1 -> T2 already set
            // (implicit with/with_mut above)
            data
        }
    }
}

impl<P1: Clone, P2: Clone, T2> Clone for Then<P1, P2, T2> {
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
    pub fn new<T1: ?Sized, T2>(get: Get, put: Put) -> Self
    where
        Get: Fn(&T1) -> Option<T2>,
        Put: Fn(&mut T1, T2),
    {
        Self { get, put }
    }
}

impl<T1: ?Sized, T2, Get, Put> PartialPrism<T1, T2> for Map<Get, Put>
where
    Get: Fn(&T1) -> Option<T2>,
    Put: Fn(&mut T1, T2),
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T2) -> V,
    {
        (&(self.get)(data)).as_ref().map(f)
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T2) -> V,
    {
        let mut temp = (self.get)(data);
        let x = temp.as_mut().map(f);
        if let Some(b) = temp {
            (self.put)(data, b);
        };
        x
    }
}

impl<T1, T2, Get, Put> Prism<T1, T2> for Map<Get, Put>
where
    Get: Fn(&T1) -> Option<T2>,
    Put: Fn(&mut T1, T2),
{
    fn replace<'a>(&self, data: &'a mut T1, v: T2) -> &'a mut T1 {
        (self.put)(data, v);
        data
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Deref;

impl<T> PartialPrism<T, T::Target> for Deref
where
    T: ?Sized + ops::Deref + ops::DerefMut,
{
    fn with<V, F>(&self, data: &T, f: F) -> Option<V>
    where
        F: FnOnce(&T::Target) -> V,
    {
        Some(f(data.deref()))
    }

    fn with_mut<V, F>(&self, data: &mut T, f: F) -> Option<V>
    where
        F: FnOnce(&mut T::Target) -> V,
    {
        Some(f(data.deref_mut()))
    }
}

impl<T> Prism<T, T::Target> for Deref
where
    T: ?Sized + ops::DerefMut,
    T::Target: Sized,
{
    fn replace<'a>(&self, data: &'a mut T, v: T::Target) -> &'a mut T {
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

impl<T, I> PartialPrism<T, T::Output> for Index<I>
where
    T: ?Sized + ops::Index<I> + ops::IndexMut<I>,
    I: Clone,
{
    fn with<V, F>(&self, data: &T, f: F) -> Option<V>
    where
        F: FnOnce(&T::Output) -> V,
    {
        Some(f(&data[self.index.clone()]))
    }
    fn with_mut<V, F>(&self, data: &mut T, f: F) -> Option<V>
    where
        F: FnOnce(&mut T::Output) -> V,
    {
        Some(f(&mut data[self.index.clone()]))
    }
}

impl<T, I> Prism<T, T::Output> for Index<I>
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Id;

impl<T: ?Sized> PartialPrism<T, T> for Id {
    fn with<V, F>(&self, data: &T, f: F) -> Option<V>
    where
        F: FnOnce(&T) -> V,
    {
        Some(f(data))
    }

    fn with_mut<V, F>(&self, data: &mut T, f: F) -> Option<V>
    where
        F: FnOnce(&mut T) -> V,
    {
        Some(f(data))
    }
}

impl<T> Prism<T, T> for Id {
    fn replace<'a>(&self, data: &'a mut T, v: T) -> &'a mut T {
        *data = v;
        data
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct InArc<P> {
    inner: P,
}

impl<P> InArc<P> {
    pub fn new<T1, T2>(inner: P) -> Self
    where
        T1: Clone,
        T2: Data,
        P: PartialPrism<T1, T2>,
    {
        Self { inner }
    }
}

impl<T1, T2, P> PartialPrism<Arc<T1>, T2> for InArc<P>
where
    T1: Clone,
    T2: Data,
    P: PartialPrism<T1, T2>,
{
    fn with<V, F>(&self, data: &Arc<T1>, f: F) -> Option<V>
    where
        F: FnOnce(&T2) -> V,
    {
        self.inner.with(data, f)
    }

    fn with_mut<V, F>(&self, data: &mut Arc<T1>, f: F) -> Option<V>
    where
        F: FnOnce(&mut T2) -> V,
    {
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

impl<T1, T2, P> Prism<Arc<T1>, T2> for InArc<P>
where
    T1: Clone + Default,
    T2: Data,
    P: Prism<T1, T2> + DefaultUpgrade<T1, T2>,
    Arc<T1>: ops::DerefMut,
{
    fn replace<'a>(&self, data: &'a mut Arc<T1>, v: T2) -> &'a mut Arc<T1> {
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
