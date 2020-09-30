// Copyright 2020 The Druid Authors.
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

//! A [`Widget`] that uses a [`Lens`] to change the [`Data`] of its child.
//!
//! [`Widget`]: ../trait.Widget.html
//! [`Lens`]: ../trait.Lens.html
//! [`Data`]: ../trait.Data.html

use std::marker::PhantomData;

use crate::widget::prelude::*;
use crate::{Data, Lens};

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
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with_mut(data, |data| inner.event(ctx, event, data, env))
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.lifecycle(ctx, event, data, env))
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        let inner = &mut self.inner;
        let lens = &self.lens;
        lens.with(old_data, |old_data| {
            lens.with(data, |data| {
                if ctx.has_requested_update() || !old_data.same(data) || ctx.env_changed() {
                    inner.update(ctx, old_data, data, env);
                }
            })
        })
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let inner = &mut self.inner;
        self.lens
            .with(data, |data| inner.layout(ctx, bc, data, env))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let inner = &mut self.inner;
        self.lens.with(data, |data| inner.paint(ctx, data, env));
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
