use crate::optics::Lens;
pub use crate::optics::Prism as Traversal;

use std::marker::PhantomData;
use std::ops;
use std::sync::Arc;

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::Data;

#[derive(Debug)]
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

#[derive(Debug, Copy)]
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

#[derive(Debug, Copy)]
pub struct BeforeLens<T, U, B: ?Sized> {
    left: T,
    right: U,
    _marker: PhantomData<B>,
}

impl<T, U, B: ?Sized> BeforeLens<T, U, B> {
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

impl<T, U, A, B, C> Traversal<A, C> for BeforeLens<T, U, B>
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

impl<T: Clone, U: Clone, B> Clone for BeforeLens<T, U, B> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
            _marker: PhantomData,
        }
    }
}
