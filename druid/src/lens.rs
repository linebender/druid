// Copyright 2019 The xi-editor Authors.
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

//! Support for lenses, a way of focusing on subfields of data.

use std::marker::PhantomData;

pub use druid_derive::Lens;

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, UpdateCtx,
    Widget,
};

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
pub trait Lens<T: ?Sized, U: ?Sized> {
    /// Get non-mut access to the field.
    ///
    /// Runs the supplied closure with a reference to the data. It's
    /// structured this way, as opposed to simply returning a reference,
    /// so that the data might be synthesized on-the-fly by the lens.
    fn with<V, F: FnOnce(&U) -> V>(&self, data: &T, f: F) -> V;

    /// Get mutable access to the field.
    ///
    /// This method is defined in terms of a closure, rather than simply
    /// yielding a mutable reference, because it is intended to be used
    /// with value-type data (also known as immutable data structures).
    /// For example, a lens for an immutable list might be implemented by
    /// cloning the list, giving the closure mutable access to the clone,
    /// then updating the reference after the closure returns.
    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> V;
}

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

impl<T, U, L, W> Widget<T> for LensWrap<U, L, W>
where
    T: Data,
    U: Data,
    L: Lens<T, U>,
    W: Widget<U>,
{
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.paint(paint_ctx, base_state, data, env));
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.layout(ctx, bc, data, env))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with_mut(data, |data| inner.event(ctx, event, data, env))
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        let inner = &mut self.inner;
        let lens = &self.lens;
        if let Some(old_data) = old_data {
            lens.with(old_data, |old_data| {
                lens.with(data, |data| {
                    if !old_data.same(data) {
                        inner.update(ctx, Some(old_data), data, env);
                    }
                })
            })
        } else {
            lens.with(data, |data| inner.update(ctx, None, data, env));
        }
    }
}
