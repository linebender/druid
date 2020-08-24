use crate::optics::{prism, Lens};

use std::marker::PhantomData;

// TODO: rename that Prism to AffineTraversal, and bring it here?
// or leave it there as PartialPrism? (or both?)
// TODO: a trait re-export really shouldn't be happening..
// TODO: rename Traversal to AffineTraversal?
// since affine means 0-or-1 (same as Option<T>), and Traversal
// may be 0-or-1-or-many.
pub use crate::optics::Prism as Traversal;

#[derive(Debug, PartialEq)]
pub struct LensWrap<L> {
    inner: L,
}

impl<L> LensWrap<L> {
    pub fn new<S: ?Sized, A: ?Sized>(inner: L) -> Self
    where
        L: Lens<S, A>,
    {
        Self { inner }
    }
}

impl<L, S, A> Traversal<S, A> for LensWrap<L>
where
    S: ?Sized,
    A: ?Sized,
    L: Lens<S, A>,
{
    fn with<V, F: FnOnce(&A) -> V>(&self, data: &S, f: F) -> Option<V> {
        Some(self.inner.with(data, f))
    }

    fn with_mut<V, F: FnOnce(&mut A) -> V>(&self, data: &mut S, f: F) -> Option<V> {
        Some(self.inner.with_mut(data, f))
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct ThenLens<T, U, B: ?Sized> {
    left: T,
    right: U,
    _marker: PhantomData<B>,
}

impl<T, U, B: ?Sized> ThenLens<T, U, B> {
    pub fn new<A: ?Sized, C: ?Sized>(left: T, right: U) -> Self
    where
        T: Traversal<A, B>,
        U: Lens<B, C>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<T, U, A, B, C> Traversal<A, C> for ThenLens<T, U, B>
where
    A: ?Sized,
    B: ?Sized,
    C: ?Sized,
    T: Traversal<A, B>,
    U: Lens<B, C>,
{
    fn with<V, F: FnOnce(&C) -> V>(&self, data: &A, f: F) -> Option<V> {
        self.left.with(data, |b| self.right.with(b, f))
    }

    fn with_mut<V, F: FnOnce(&mut C) -> V>(&self, data: &mut A, f: F) -> Option<V> {
        self.left.with_mut(data, |b| self.right.with_mut(b, f))
    }
}

impl<T: Clone, U: Clone, B> Clone for ThenLens<T, U, B> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

// TODO: decide if this should exist..
impl<T, U, A, B, C> prism::PrismReplacer<A, C> for ThenLens<T, U, B>
where
    A: ?Sized + Default,
    B: ?Sized + Default,
    C: Sized + Clone,
    T: prism::Prism<A, B> + prism::PrismReplacer<A, B>,
    U: Lens<B, C>,
{
    /// Given the matching path of `A` -> `B` -> `C`,
    /// it is guaranteed that `A` will end up matching
    /// to `B`.
    ///
    ///
    /// ---------------------------------
    ///
    /// First it tries replacing `B` -> `C`, which would
    /// mean that `A` -> `B` is already in place.
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
                let () = self.right.with_mut(b, |c| *c = v.clone());
                ()
            },
        );
        if some_replacement.is_none() {
            // couldn't access A -> B,
            // give up the replacement
            // and build B -> C from scratch
            let mut new_b = B::default();
            let () = self.right.with_mut(&mut new_b, |c| *c = v);

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
        self.left.upgrade({
            // build B -> C from scratch
            let mut new_b = B::default();
            let () = self.right.with_mut(&mut new_b, |c| *c = v);

            new_b
        })
    }
}

#[derive(Debug, Copy, PartialEq)]
pub struct AfterLens<T, U, B: ?Sized> {
    left: T,
    right: U,
    _marker: PhantomData<B>,
}

impl<T, U, B: ?Sized> AfterLens<T, U, B> {
    pub fn new<A: ?Sized, C: ?Sized>(left: T, right: U) -> Self
    where
        T: Lens<A, B>,
        U: Traversal<B, C>,
    {
        Self {
            left,
            right,
            _marker: PhantomData,
        }
    }
}

impl<T, U, A, B, C> Traversal<A, C> for AfterLens<T, U, B>
where
    A: ?Sized,
    B: ?Sized,
    C: ?Sized,
    T: Lens<A, B>,
    U: Traversal<B, C>,
{
    fn with<V, F: FnOnce(&C) -> V>(&self, data: &A, f: F) -> Option<V> {
        self.left.with(data, |b| self.right.with(b, f))
    }

    fn with_mut<V, F: FnOnce(&mut C) -> V>(&self, data: &mut A, f: F) -> Option<V> {
        self.left.with_mut(data, |b| self.right.with_mut(b, f))
    }
}

impl<T: Clone, U: Clone, B> Clone for AfterLens<T, U, B> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}

// TODO: decide if this should exist..
impl<T, U, A, B, C> prism::PrismReplacer<A, C> for AfterLens<T, U, B>
where
    A: ?Sized + Default,
    B: ?Sized + Default,
    C: Sized + Clone,
    T: Lens<A, B>,
    U: Traversal<B, C> + prism::PrismReplacer<B, C>,
{
    /// Given the matching path of `A` -> `B` -> `C`,
    /// it is guaranteed that `B` will end up matching
    /// to `C`.
    ///
    /// It only forwards the replacement into `B` -> `C`.
    fn replace<'a>(&self, data: &'a mut A, v: C) -> &'a mut A {
        #[allow(clippy::unused_unit)]
        let () = self.left.with_mut(
            data,
            // A -> B was already set
            // only replaces B -> C
            // (as A -> B is already set)
            |b| {
                self.right.replace(b, v);
                ()
            },
        );
        data
    }

    fn upgrade<'a>(&self, v: C) -> A
    where
        A: Sized,
    {
        let mut new_a = A::default();
        let () = self
            .left
            .with_mut(&mut new_a, |b| *b = self.right.upgrade(v));
        new_a
    }
}
