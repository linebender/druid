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

use std::fmt::Display;
use std::mem;
use std::str::FromStr;

use crate::widget::prelude::*;
use crate::Data;

/// Converts a `Widget<String>` to a `Widget<Option<T>>`, mapping parse errors to None
pub struct Parse<T> {
    widget: T,
    // because we are synthesizing the data for our inner widget, we keep track
    // of 'old' and 'new' data as widgetpod would, so that our child can handle
    // update() correctly.
    prev_state: String,
    state: String,
}

impl<T> Parse<T> {
    /// Create a new `Parse` widget.
    pub fn new(widget: T) -> Self {
        Self {
            widget,
            prev_state: String::new(),
            state: String::new(),
        }
    }
}

impl<T: FromStr + Display + Data, W: Widget<String>> Widget<Option<T>> for Parse<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        self.widget.event(ctx, event, &mut self.state, env);
        if self.state != self.prev_state {
            *data = self.state.parse().ok();
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Option<T>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if let Some(data) = data {
                self.state = data.to_string();
                self.prev_state = self.state.clone();
            }
        }
        self.widget.lifecycle(ctx, event, &self.state, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Option<T>, data: &Option<T>, env: &Env) {
        if !old_data.same(data) {
            if let Some(data) = data {
                // only reset our state if the data has changed; this means we
                // do *not* reset it, for instance, when the text changes from
                // "42" to "42." (when parsing a float) but it *will* when going
                // from "42.2" to "42.". This is an annoying limitation of this
                // implementation.
                self.prev_state = mem::replace(&mut self.state, data.to_string());
            }
        }
        self.widget.update(ctx, &self.prev_state, &self.state, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &Option<T>,
        env: &Env,
    ) -> Size {
        self.widget.layout(ctx, bc, &self.state, env)
    }

    fn paint(&mut self, paint: &mut PaintCtx, _data: &Option<T>, env: &Env) {
        self.widget.paint(paint, &self.state, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.widget.id()
    }
}
