// Copyright 2020 The xi-editor Authors.
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

use crate::{widget::prelude::*, Data};

/// A widget that allows writing parts of the ui in an immediate-mode style.
///
/// The primary use case for this is displaying data that doesn't fit into druid's model, such as
/// enums you have no control over.
///
/// Whenever the state represented by the `Immediate` changes,
/// it will reconstruct its content for the new data.
///
/// While this is not the most efficient thing to do, it is very simple and performs perfectly fine
/// for small or rarely changed data.
///
/// You should only use `Immediate` if your data format can't be reasonably used with other widgets.
pub struct Immediate<D, W: Widget<()>> {
    constructor: Box<dyn Fn(&D) -> W>,
    content: Option<W>,
}

impl<D, W: Widget<()>> Immediate<D, W> {
    /// Takes a constructor for a stateless widget
    pub fn new(constructor: impl Fn(&D) -> W + 'static) -> Self {
        Self {
            constructor: Box::new(constructor),
            content: None,
        }
    }
}

impl<D: Data, W: Widget<()>> Widget<D> for Immediate<D, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut D, env: &Env) {
        if let Some(content) = &mut self.content {
            content.event(ctx, event, &mut (), env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &D, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.content = Some((self.constructor)(data));
        }
        if let Some(content) = &mut self.content {
            content.lifecycle(ctx, event, &(), env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &D, data: &D, env: &Env) {
        if !old_data.same(data) {
            self.content = Some((self.constructor)(data));
            ctx.children_changed();
        } else {
            // This can happen when env changes, right?
            if let Some(content) = &mut self.content {
                content.update(ctx, &(), &(), env);
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &D, env: &Env) -> Size {
        if let Some(content) = &mut self.content {
            content.layout(ctx, bc, &(), env)
        } else {
            Size::ZERO
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &D, env: &Env) {
        if let Some(content) = &mut self.content {
            content.paint(ctx, &(), env);
        }
    }
}
