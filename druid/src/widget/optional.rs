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

//! A widget that displays only when an `Option` is `Some`.

use crate::widget::prelude::*;
use crate::{Data, Point, WidgetPod};

/// A widget that displays only when an `Option` is `Some`.
///
/// If you want to display a widget in the `None` case, wrap this in an `Either` widget.
pub struct Optional<T> {
    inner: WidgetPod<T, Box<dyn Widget<T>>>,
    /// Keep track of whether we've done 'WidgetAdded'. We can't do this until we get `Some` data
    /// for the first time.
    init: bool,
}

impl<T> Optional<T> {
    /// Create a new widget that only shows when data is `Some`.
    pub fn new(inner: impl Widget<T> + 'static) -> Optional<T> {
        Optional {
            inner: WidgetPod::new(inner).boxed(),
            init: false,
        }
    }
}

impl<T: Data> Widget<Option<T>> for Optional<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        if let Some(data) = data.as_mut() {
            self.inner.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Option<T>,
        env: &Env,
    ) {
        if let Some(data) = data.as_ref() {
            self.inner.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Option<T>, data: &Option<T>, env: &Env) {
        if old_data.is_some() != data.is_some() {
            ctx.request_layout();
        }
        if let Some(data) = data.as_ref() {
            self.inner.update(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Option<T>,
        env: &Env,
    ) -> Size {
        if let Some(data) = data.as_ref() {
            let size = self.inner.layout(ctx, bc, data, env);
            self.inner.set_origin(ctx, data, env, Point::ORIGIN);
            size
        } else {
            bc.min()
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Option<T>, env: &Env) {
        if let Some(data) = data.as_ref() {
            self.inner.paint(ctx, data, env);
        }
    }
}
