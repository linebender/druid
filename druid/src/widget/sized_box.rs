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

//! A widget with predefined size.

use std::f64::INFINITY;

use crate::shell::kurbo::Size;
use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

/// A widget with predefined size.
///
/// If given a child, this widget forces its child to have a specific width and/or height
/// (assuming values are permitted by this widget's parent). If either the width or height is not set,
/// this widget will size itself to match the child's size in that dimension.
///
/// If not given a child, SizedBox will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
pub struct SizedBox<T: Data> {
    inner: Option<Box<dyn Widget<T>>>,
    width: Option<f64>,
    height: Option<f64>,
}

impl<T: Data> SizedBox<T> {
    /// Construct container with child, and both width and height not set.
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            inner: Some(Box::new(inner)),
            width: None,
            height: None,
        }
    }

    /// Construct container without child, and both width and height not set.
    pub fn empty() -> Self {
        Self {
            inner: None,
            width: None,
            height: None,
        }
    }

    /// Set container's width.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// Expand container to fit the parent.
    /// It is equivalent to setting width and height to Infinity.
    pub fn expand(mut self) -> Self {
        self.width = Some(INFINITY);
        self.height = Some(INFINITY);
        self
    }
}

impl<T: Data> Widget<T> for SizedBox<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.paint(paint_ctx, base_state, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("SizedBox");

        match self.inner {
            Some(ref mut inner) => {
                let (min_width, max_width) = match self.width {
                    Some(width) => {
                        let w = width.max(bc.min().width).min(bc.max().width);
                        (w, w)
                    }
                    None => (bc.min().width, bc.max().width),
                };

                let (min_height, max_height) = match self.height {
                    Some(height) => {
                        let h = height.max(bc.min().height).min(bc.max().height);
                        (h, h)
                    }
                    None => (bc.min().height, bc.max().height),
                };

                let child_bc = BoxConstraints::new(
                    Size::new(min_width, min_height),
                    Size::new(max_width, max_height),
                );

                inner.layout(ctx, &child_bc, data, env)
            }
            None => bc.constrain((self.width.unwrap_or(0.0), self.height.unwrap_or(0.0))),
        }
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.event(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.update(ctx, old_data, data, env);
        }
    }
}
