#![allow(missing_docs)]

pub use crate::optics::PartialPrism as AffineTraversal;
use crate::optics::{lens, prism, Lens};
pub use fallback::Fallback;
pub use guard::Guard;
use std::marker::PhantomData;
pub use then::Then;
pub use wrap::Wrap;

mod layer {
    #[derive(Clone, Debug, PartialEq)]
    pub struct Lens;

    #[derive(Clone, Debug, PartialEq)]
    pub struct Prism;
}

mod wrap {
    use super::{layer, lens, prism};
    use crate::widget::{LensWrap, PrismWrap};

    pub trait Wrap<Layer, T1: ?Sized, T2: ?Sized, LayerKind> {
        type Target;
        fn wrap(self, layer: Layer) -> Self::Target;
    }

    impl<L, T1, T2, W> Wrap<L, T1, T2, layer::Lens> for W
    where
        T1: ?Sized,
        T2: Sized,
        L: lens::Lens<T1, T2>,
    {
        type Target = LensWrap<T2, L, W>;
        fn wrap(self, lens: L) -> Self::Target {
            LensWrap::new(self, lens)
        }
    }

    impl<P, T1, T2, W> Wrap<P, T1, T2, layer::Prism> for W
    where
        T1: ?Sized,
        T2: Sized,
        P: prism::Prism<T1, T2>,
    {
        type Target = PrismWrap<T2, P, W>;
        fn wrap(self, prism: P) -> Self::Target {
            PrismWrap::new(self, prism)
        }
    }
}

mod then {
    use super::{layer, lens, prism};

    /// Applies a second layer after the first one.
    pub trait Then<Other, T1: ?Sized, T2: ?Sized, T3: ?Sized, LayerKind1, LayerKind2> {
        /// The composed structure for the given `LayerKind1` and `LayerKind2`.
        type Target;
        /// Composes `Self` with `Other`,
        /// so that `other` is applied after `self`.
        fn then(self, other: Other) -> Self::Target;
    }

    /// Compose a `Lens<T1, T2>` with a `Lens<T2, T3>` to produce a `Lens<T1, T3>`.
    ///
    /// ```
    /// # use druid::{optics::affine_traversal::Then, *};
    /// struct Foo { x: (u32, bool) }
    /// let lens = lens!(Foo, x).then(lens!((u32, bool), 1));
    /// assert_eq!(lens.get(&Foo { x: (0, true) }), true);
    /// ```
    impl<L1, L2, T1, T2, T3> Then<L2, T1, T2, T3, layer::Lens, layer::Lens> for L1
    where
        T1: ?Sized,
        T2: ?Sized,
        T3: ?Sized,
        L1: lens::Lens<T1, T2>,
        L2: lens::Lens<T2, T3>,
    {
        type Target = lens::Then<L1, L2, T2>;
        fn then(self, lens: L2) -> Self::Target {
            lens::Then::new(self, lens)
        }
    }

    /// Compose a `Lens<T1, T2>` with a `Prism<T2, T3>` to produce a `Prism<T1, T3>`.
    ///
    /// ```
    /// # use druid::{optics::affine_traversal::Then, *};
    /// struct Foo { x: Result<u32, bool> }
    /// let aff = lens!(Foo, x)
    ///     .then(prism!(Result<u32, bool>, Ok));
    /// assert_eq!(aff.get(&Foo { x: Ok(7) }), Some(7));
    /// assert_eq!(aff.get(&Foo { x: Err(true) }), None);
    /// ```
    impl<L1, P2, T1, T2, T3> Then<P2, T1, T2, T3, layer::Lens, layer::Prism> for L1
    where
        T1: ?Sized,
        T2: ?Sized,
        T3: ?Sized,
        L1: lens::Lens<T1, T2>,
        P2: prism::PartialPrism<T2, T3>,
    {
        type Target = super::ThenAfterLens<Self, P2, T2>;
        fn then(self, prism: P2) -> Self::Target {
            super::ThenAfterLens::new(self, prism)
        }
    }

    /// Compose a `Prism<T1, T2>` with a `Lens<T2, T3>` to produce a `Prism<T1, T3>`.
    /// ```
    /// # use druid::{optics::affine_traversal::Then, *};
    /// type Outer = Result<Inner, f32>;
    /// type Inner = (u32, bool);
    /// let aff = prism!(Outer, Ok)
    ///     .then(lens!(Inner, 1));
    /// assert_eq!(aff.get(&Outer::Ok((3, true))), Some(true));
    /// assert_eq!(aff.get(&Outer::Err(5.5)), None);
    /// ```
    impl<P1, L2, T1, T2, T3> Then<L2, T1, T2, T3, layer::Prism, layer::Lens> for P1
    where
        T1: ?Sized,
        T2: ?Sized,
        T3: ?Sized,
        P1: prism::PartialPrism<T1, T2>,
        L2: lens::Lens<T2, T3>,
    {
        type Target = super::ThenLens<P1, L2, T2>;
        fn then(self, lens: L2) -> Self::Target {
            super::ThenLens::new(self, lens)
        }
    }

    /// Compose a `Prism<T1, T2>` with a `Prism<T2, T3>` to produce a `Prism<T1, T3>`.
    /// ```
    /// # use druid::{optics::affine_traversal::Then, *};
    /// type Outer = Result<Inner, f32>;
    /// type Inner = Result<u32, bool>;
    /// let aff = prism!(Outer, Ok)
    ///     .then(prism!(Inner, Err));
    /// assert_eq!(aff.get(&Outer::Ok(Inner::Err(true))), Some(true));
    /// assert_eq!(aff.get(&Outer::Err(5.5)), None);
    /// assert_eq!(aff.get(&Outer::Ok(Inner::Ok(1u32))), None);
    /// ```
    impl<P1, P2, T1, T2, T3> Then<P2, T1, T2, T3, layer::Prism, layer::Prism> for P1
    where
        T1: ?Sized,
        T2: ?Sized,
        T3: ?Sized,
        P1: prism::PartialPrism<T1, T2>,
        P2: prism::PartialPrism<T2, T3>,
    {
        type Target = prism::Then<P1, P2, T2>;
        fn then(self, prism: P2) -> Self::Target {
            prism::Then::new(self, prism)
        }
    }
}

mod guard {
    use super::{layer, lens, prism};

    /// Uses a `Prism` to guard a `Lens`.
    /// Ie. turns a `Lens` inaccessible when a `Prism` is also inaccessible.
    pub trait Guard<Other, T1: ?Sized, T2: ?Sized, T3: ?Sized, LayerKind1, LayerKind2> {
        /// The composed structure for the given `LayerKind1` and `LayerKind2`.
        type Target;
        /// Guards a lens.
        ///
        /// Composes `Self` with `Other`,
        /// so that `other` is directly applied if `self` is accessible.
        fn guard(self, other: Other) -> Self::Target;
    }

    /// Compose a `Prism<T1, T2>` with a `Lens<T1, T3>` to produce a `Prism<T1, T3>`.
    impl<P1, L2, T1, T2, T3> Guard<L2, T1, T2, T3, layer::Prism, layer::Lens> for P1
    where
        T1: ?Sized,
        T2: ?Sized,
        T3: ?Sized,
        P1: prism::PartialPrism<T1, T2>,
        L2: lens::Lens<T1, T3>,
    {
        type Target = super::PrismGuard<P1, L2, T2>;
        fn guard(self, lens: L2) -> Self::Target {
            super::PrismGuard::new(self, lens)
        }
    }
}

mod fallback {
    use super::{layer, lens, prism};

    /// Use `Other` as a fallback for a `Prism` (`Self`).
    pub trait Fallback<Other, T1: ?Sized, T2: ?Sized, LayerKind1, LayerKind2> {
        /// The composed structure for the given `LayerKind1` and `LayerKind2`.
        type Target;
        /// Composes `Self` with `Other`,
        /// so that `other` is directly applied if `self` is not accessible.
        /// Otherwise, `self` is normally and directly applied.
        fn with_fallback(self, other: Other) -> Self::Target;
    }

    /// Compose a `Prism<T1, T2>` with a `Lens<T1, T2>` to produce a `Lens<T1, T2>`.
    ///
    /// If `T2` is not accessible from the `Prism`, it is then accessed
    /// from the `Lens`.
    impl<P1, L2, T1, T2> Fallback<L2, T1, T2, layer::Prism, layer::Lens> for P1
    where
        T1: ?Sized,
        T2: ?Sized,
        P1: prism::PartialPrism<T1, T2>,
        L2: lens::Lens<T1, T2>,
    {
        type Target = super::OrMapLens<P1, L2>;
        fn with_fallback(self, lens: L2) -> Self::Target {
            super::OrMapLens::new(self, lens)
        }
    }

    /// Compose a `Prism<T1, T2>` with a `Prism<T1, T2>` to produce a `Prism<T1, T2>`.
    ///
    /// If `T2` is not accessible from the first `Prism`,
    /// the second `Prism`is tried.
    impl<P1, P2, T1, T2> Fallback<P2, T1, T2, layer::Prism, layer::Prism> for P1
    where
        T1: ?Sized,
        T2: ?Sized,
        P1: prism::PartialPrism<T1, T2>,
        P2: prism::PartialPrism<T1, T2>,
    {
        type Target = super::OrMapPrism<P1, P2>;
        fn with_fallback(self, prism: P2) -> Self::Target {
            super::OrMapPrism::new(self, prism)
        }
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct ThenLens<P1, L2, T2: ?Sized> {
    left: P1,
    right: L2,
    _marker: PhantomData<T2>,
}

impl<P1, L2, T2: ?Sized> ThenLens<P1, L2, T2> {
    pub fn new<T1: ?Sized, T3: ?Sized>(left: P1, right: L2) -> Self
    where
        P1: AffineTraversal<T1, T2>,
        L2: Lens<T2, T3>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<P1, L2, T1, T2, T3> AffineTraversal<T1, T3> for ThenLens<P1, L2, T2>
where
    T1: ?Sized,
    T2: ?Sized,
    T3: ?Sized,
    P1: AffineTraversal<T1, T2>,
    L2: Lens<T2, T3>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T3) -> V,
    {
        self.left.with(data, |t2| self.right.with(t2, f))
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T3) -> V,
    {
        self.left.with_mut(data, |t2| self.right.with_mut(t2, f))
    }
}

impl<P1: Clone, L2: Clone, T2> Clone for ThenLens<P1, L2, T2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

// TODO: decide if this should exist..
impl<P1, L2, T1, T2, T3> prism::Prism<T1, T3> for ThenLens<P1, L2, T2>
where
    T1: ?Sized + Default,
    T2: ?Sized + Default,
    T3: Sized + Clone,
    P1: prism::PartialPrism<T1, T2> + prism::Prism<T1, T2>,
    L2: Lens<T2, T3>,
{
    fn replace<'a>(&self, base: &'a mut T1, v: T3) -> &'a mut T1
    where
        T1: Sized,
    {
        self.left.replace(base, {
            // build T2 -> T3 from scratch
            let mut new_t2 = T2::default();
            let () = self.right.with_mut(&mut new_t2, |t3| *t3 = v);

            new_t2
        })
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct OrMapLens<P1, L2> {
    left: P1,
    right: L2,
}

impl<P1, L2> OrMapLens<P1, L2> {
    pub fn new<T1: ?Sized, T2: ?Sized>(left: P1, right: L2) -> Self
    where
        P1: AffineTraversal<T1, T2>,
        L2: Lens<T1, T2>,
    {
        Self { left, right }
    }
}

impl<P1, L2, T1, T2> druid::lens::Lens<T1, T2> for OrMapLens<P1, L2>
where
    T1: ?Sized,
    T2: ?Sized,
    P1: AffineTraversal<T1, T2>,
    L2: Lens<T1, T2>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> V
    where
        F: FnOnce(&T2) -> V,
    {
        if let Some(()) = self.left.with(data, |_t2: &T2| ()) {
            self.left.with(data, f).unwrap()
        } else {
            self.right.with(data, f)
        }
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> V
    where
        F: FnOnce(&mut T2) -> V,
    {
        if let Some(()) = self.left.with_mut(data, |_t2: &mut T2| ()) {
            self.left.with_mut(data, f).unwrap()
        } else {
            self.right.with_mut(data, f)
        }
    }
}

impl<P1: Clone, L2: Clone> Clone for OrMapLens<P1, L2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct OrMapPrism<P1, P2> {
    left: P1,
    right: P2,
}

impl<P1, P2> OrMapPrism<P1, P2> {
    pub fn new<T1: ?Sized, T2: ?Sized>(left: P1, right: P2) -> Self
    where
        P1: AffineTraversal<T1, T2>,
        P2: AffineTraversal<T1, T2>,
    {
        Self { left, right }
    }
}

impl<P1, P2, T1, T2> AffineTraversal<T1, T2> for OrMapPrism<P1, P2>
where
    T1: ?Sized,
    T2: ?Sized,
    P1: AffineTraversal<T1, T2>,
    P2: AffineTraversal<T1, T2>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T2) -> V,
    {
        if let Some(()) = self.left.with(data, |_t2: &T2| ()) {
            self.left.with(data, f)
        } else {
            self.right.with(data, f)
        }
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T2) -> V,
    {
        if let Some(()) = self.left.with_mut(data, |_t2: &mut T2| ()) {
            self.left.with_mut(data, f)
        } else {
            self.right.with_mut(data, f)
        }
    }
}

impl<P1: Clone, L2: Clone> Clone for OrMapPrism<P1, L2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct PrismGuard<P1, L2, T2: ?Sized> {
    left: P1,
    right: L2,
    _marker: PhantomData<T2>,
}

impl<P1, L2, T2: ?Sized> PrismGuard<P1, L2, T2> {
    pub fn new<T1: ?Sized, T3: ?Sized>(left: P1, right: L2) -> Self
    where
        P1: AffineTraversal<T1, T2>,
        L2: Lens<T1, T3>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<P1, L2, T1, T2, T3> AffineTraversal<T1, T3> for PrismGuard<P1, L2, T2>
where
    T1: ?Sized,
    T2: ?Sized,
    T3: ?Sized,
    P1: AffineTraversal<T1, T2>,
    L2: Lens<T1, T3>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T3) -> V,
    {
        self.left
            .with(data, |_t2| ())
            .map(|_: ()| self.right.with(data, f))
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T3) -> V,
    {
        self.left
            .with_mut(data, |_t2| ())
            .map(|_: ()| self.right.with_mut(data, f))
    }
}

impl<P1: Clone, L2: Clone, T2> Clone for PrismGuard<P1, L2, T2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct ThenAfterLens<L1, P2, T2: ?Sized> {
    left: L1,
    right: P2,
    _marker: PhantomData<T2>,
}

impl<L1, P2, T2: ?Sized> ThenAfterLens<L1, P2, T2> {
    pub fn new<T1: ?Sized, T3: ?Sized>(left: L1, right: P2) -> Self
    where
        L1: Lens<T1, T2>,
        P2: AffineTraversal<T2, T3>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<L1, P2, T1, T2, T3> AffineTraversal<T1, T3> for ThenAfterLens<L1, P2, T2>
where
    T1: ?Sized,
    T2: ?Sized,
    T3: ?Sized,
    L1: Lens<T1, T2>,
    P2: AffineTraversal<T2, T3>,
{
    fn with<V, F>(&self, data: &T1, f: F) -> Option<V>
    where
        F: FnOnce(&T3) -> V,
    {
        self.left.with(data, |t2| self.right.with(t2, f))
    }

    fn with_mut<V, F>(&self, data: &mut T1, f: F) -> Option<V>
    where
        F: FnOnce(&mut T3) -> V,
    {
        self.left.with_mut(data, |t2| self.right.with_mut(t2, f))
    }
}

impl<L1: Clone, P2: Clone, T2> Clone for ThenAfterLens<L1, P2, T2> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

// TODO: decide if this should exist..
impl<L1, P2, T1, T2, T3> prism::Prism<T1, T3> for ThenAfterLens<L1, P2, T2>
where
    T1: ?Sized + Default,
    T2: ?Sized + Default,
    T3: Sized + Clone,
    L1: Lens<T1, T2>,
    P2: AffineTraversal<T2, T3> + prism::Prism<T2, T3>,
{
    /// Given the matching path of `T1` -> `T2` -> `T3`,
    /// it is guaranteed that `T2` will end up matching
    /// to `T3`.
    ///
    /// It only forwards the replacement into `T2` -> `T3`.
    fn replace<'a>(&self, data: &'a mut T1, v: T3) -> &'a mut T1 {
        #[allow(clippy::unused_unit)]
        let () = self.left.with_mut(
            data,
            // T1 -> T2 was already set
            // only replaces T2 -> T3
            // (as T1 -> T2 is already set)
            |t2| {
                self.right.replace(t2, v);
                ()
            },
        );
        data
    }
}
