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

//! A helper widget for parsing data into text.

use std::fmt::Display;
use std::mem;
use std::str::FromStr;

use crate::prelude::*;
use crate::{BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget};

/// Converts a `Widget<String>` to a `Widget<Option<T>>`, mapping parse errors to None
pub struct Parse<T> {
    widget: T,
    state: String,
}

impl<T> Parse<T> {
    pub fn new(widget: T) -> Self {
        Self {
            widget,
            state: String::new(),
        }
    }
}

impl<T: FromStr + Display + Data, W: Widget<String>> Widget<Option<T>> for Parse<W> {
    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: Option<&Option<T>>,
        data: &Option<T>,
        env: &Env,
    ) {
        let old = match *data {
            None => return, // Don't clobber the input
            Some(ref x) => mem::replace(&mut self.state, x.to_string()),
        };
        let old = old_data.map(|_| old);
        self.widget.update(ctx, old.as_ref(), &self.state, env)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        self.widget.event(ctx, event, &mut self.state, env);
        *data = self.state.parse().ok();
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
}
