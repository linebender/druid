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

use std::sync::atomic::{AtomicU32, Ordering};

use crate::kurbo::Size;

use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget, WindowId,
};

/// The target of a command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Window(WindowId),
    Widget(WidgetId),
}

impl Target {
    pub(crate) fn is_window(self) -> bool {
        match self {
            Target::Window(_) => true,
            _ => false,
        }
    }
}

impl From<WindowId> for Target {
    fn from(id: WindowId) -> Target {
        Target::Window(id)
    }
}

impl From<WidgetId> for Target {
    fn from(id: WidgetId) -> Target {
        Target::Widget(id)
    }
}

// We might decide to clean this up; it's to make it easy to offer different
// arguments to submit_command.
pub trait IntoOptTarget {
    fn into_opt_target(self) -> Option<Target>;
}

impl IntoOptTarget for Target {
    fn into_opt_target(self) -> Option<Target> {
        Some(self)
    }
}

impl IntoOptTarget for Option<Target> {
    fn into_opt_target(self) -> Option<Target> {
        self
    }
}

impl IntoOptTarget for WindowId {
    fn into_opt_target(self) -> Option<Target> {
        Some(Target::Window(self))
    }
}

impl IntoOptTarget for WidgetId {
    fn into_opt_target(self) -> Option<Target> {
        Some(Target::Widget(self))
    }
}

/// A unique identifier for a single widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetId(u32);

static WIDGET_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

impl WidgetId {
    /// Allocate a new, unique widget id.
    ///
    /// Do note that if we create 4 billion widgets there may be a collision.
    pub(crate) fn next() -> WidgetId {
        let id = WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        WidgetId(id)
    }
}

/// A wrapper that adds an identity to an otherwise anonymous widget.
pub struct IdentityWrapper<W> {
    id: WidgetId,
    inner: W,
}

impl<W> IdentityWrapper<W> {
    /// Assign an identity to a widget.
    pub fn wrap(inner: W) -> (WidgetId, IdentityWrapper<W>) {
        let id = WidgetId::next();
        (id, IdentityWrapper { id, inner })
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for IdentityWrapper<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(paint_ctx, data, env);
    }

    fn get_id(&self) -> Option<WidgetId> {
        Some(self.id)
    }
}
