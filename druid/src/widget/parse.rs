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
use tracing::instrument;

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::Data;

/// Converts a `Widget<String>` to a `Widget<Option<T>>`, mapping parse errors to None
pub struct Parse<T> {
    widget: T,
    state: String,
}

impl<T> Parse<T> {
    /// Create a new `Parse` widget.
    pub fn new(widget: T) -> Self {
        Self {
            widget,
            state: String::new(),
        }
    }
}

impl<T: FromStr + Display + Data, W: Widget<String>> Widget<Option<T>> for Parse<W> {
    #[instrument(name = "Parse", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        self.widget.event(ctx, event, &mut self.state, env);
        *data = self.state.parse().ok();
    }

    #[instrument(name = "Parse", level = "trace", skip(self, ctx, event, data, env))]
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
            }
        }
        self.widget.lifecycle(ctx, event, &self.state, env)
    }

    #[instrument(name = "Parse", level = "trace", skip(self, ctx, _old_data, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &Option<T>, data: &Option<T>, env: &Env) {
        let old = match *data {
            None => return, // Don't clobber the input
            Some(ref x) => mem::replace(&mut self.state, x.to_string()),
        };
        self.widget.update(ctx, &old, &self.state, env)
    }

    #[instrument(name = "Parse", level = "trace", skip(self, ctx, bc, _data, env))]
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &Option<T>,
        env: &Env,
    ) -> Size {
        self.widget.layout(ctx, bc, &self.state, env)
    }

    #[instrument(name = "Parse", level = "trace", skip(self, ctx, _data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &Option<T>, env: &Env) {
        self.widget.paint(ctx, &self.state, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.widget.id()
    }

    fn debug_state(&self, _data: &Option<T>) -> DebugState {
        DebugState {
            display_name: "Parse".to_string(),
            main_value: self.state.clone(),
            ..Default::default()
        }
    }
}
