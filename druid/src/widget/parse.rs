// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

// This whole widget was deprecated in Druid 0.7
// https://github.com/linebender/druid/pull/1377
#![allow(deprecated)]

use std::fmt::Display;
use std::mem;
use std::str::FromStr;
use tracing::instrument;

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::Data;

/// Converts a `Widget<String>` to a `Widget<Option<T>>`, mapping parse errors to None
#[doc(hidden)]
#[deprecated(since = "0.7.0", note = "Use the Formatter trait instead")]
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
            Some(ref x) => {
                // Its possible that the current self.state already represents the data value
                // in that case we shouldn't clobber the self.state. This helps deal
                // with types where parse()/to_string() round trips can lose information
                // e.g. with floating point numbers, text of "1.0" becomes "1" in the
                // round trip, and this makes it impossible to type in the . otherwise
                match self.state.parse() {
                    Err(_) => Some(mem::replace(&mut self.state, x.to_string())),
                    Ok(v) => {
                        if !Data::same(&v, x) {
                            Some(mem::replace(&mut self.state, x.to_string()))
                        } else {
                            None
                        }
                    }
                }
            }
        };
        // if old is None here, that means that self.state hasn't changed
        let old_data = old.as_ref().unwrap_or(&self.state);
        self.widget.update(ctx, old_data, &self.state, env)
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
