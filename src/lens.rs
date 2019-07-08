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

use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size,
    UpdateCtx, Widget,
};

/// A lens is a datatype that gives access to a field within a larger
/// data structure.
pub trait Lens<T, U> {
    /// Get non-mut access to the field.
    fn get<'a>(&self, data: &'a T) -> &'a U;

    /// Get mutable access to the field.
    ///
    /// Discussion: I'm not 100% sure this needs to be laundered through
    /// a closure (and that `get` doesn't).
    fn with_mut<V, F: FnOnce(&mut U) -> V>(&self, data: &mut T, f: F) -> V;
}

// A case can be made this should be in the `widget` module.

pub struct LensWrap<U, L, W> {
    inner: W,
    lens: L,
    // The following is a workaround for otherwise getting E0207.
    phantom: PhantomData<U>,
}

impl<U, L, W> LensWrap<U, L, W> {
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
        self.inner
            .paint(paint_ctx, base_state, self.lens.get(data), env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, self.lens.get(data), env)
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        let inner = &mut self.inner;
        self.lens
            .with_mut(data, |data| inner.event(event, ctx, data, env))
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        if let Some(old_data) = old_data {
            if self.lens.get(old_data).same(self.lens.get(data)) {
                return;
            }
        }
        self.inner.update(
            ctx,
            old_data.map(|old_data| self.lens.get(old_data)),
            self.lens.get(data),
            env,
        );
    }
}
