// Copyright 2019 The Druid Authors.
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

use crate::kurbo::Size;
use crate::widget::prelude::*;
use crate::widget::WidgetWrapper;
use crate::Data;
use tracing::instrument;

/// A wrapper that adds an identity to an otherwise anonymous widget.
pub struct IdentityWrapper<W> {
    id: WidgetId,
    inner: W,
}

impl<W> IdentityWrapper<W> {
    /// Assign an identity to a widget.
    pub fn wrap(inner: W, id: WidgetId) -> IdentityWrapper<W> {
        IdentityWrapper { id, inner }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for IdentityWrapper<W> {
    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    #[instrument(
        name = "IdentityWrapper",
        level = "trace",
        skip(self, ctx, bc, data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    #[instrument(name = "IdentityWrapper", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }
}

impl<W> WidgetWrapper for IdentityWrapper<W> {
    widget_wrapper_body!(W, inner);
}
