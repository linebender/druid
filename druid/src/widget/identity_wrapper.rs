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

//! A widget that provides an explicit identity to a child.

use std::marker::PhantomData;

use crate::kurbo::Size;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetId,
};

/// A wrapper that adds an identity to an otherwise anonymous widget.
pub struct IdentityWrapper<T, W> {
    id: WidgetId,
    inner: W,
    phantom: PhantomData<T>,
}

impl<T: Data, W: Widget<T>> IdentityWrapper<T, W> {
    /// Assign an identity to a widget.
    pub fn wrap(inner: W) -> (WidgetId, IdentityWrapper<T, W>) {
        // if the inner widget already has an id (for instance if it uses
        // a WidgetPod) then we reuse that.
        let id = inner.id().unwrap_or_else(WidgetId::next);
        (
            id,
            IdentityWrapper {
                id,
                inner,
                phantom: PhantomData,
            },
        )
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for IdentityWrapper<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{Label, WidgetExt};
    use crate::Color;

    #[test]
    fn test_nesting() {
        let (id, label) = IdentityWrapper::wrap(Label::<u32>::new("howdy there friend"));
        let wrapped_up: Box<dyn Widget<u32>> =
            Box::new(label.padding(5.0).align_left().background(Color::BLACK));

        assert_eq!(wrapped_up.id(), Some(id));
    }
}
