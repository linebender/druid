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

//! A widget that provides simple visual styling options to a child.

use crate::shell::kurbo::{Point, Rect, RoundedRect, Size};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintBrush,
    PaintCtx, RenderContext, UpdateCtx, Widget, WidgetPod,
};

struct BorderStyle {
    width: f64,
    brush: PaintBrush,
}

/// A widget that provides simple visual styling options to a child.
pub struct Container<T: Data> {
    background: Option<PaintBrush>,
    border: Option<BorderStyle>,
    corner_radius: f64,

    inner: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Container<T> {
    /// Create Container with a child
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            background: None,
            border: None,
            corner_radius: 0.0,
            inner: WidgetPod::new(inner).boxed(),
        }
    }

    /// Paint background with a color or a gradient.
    pub fn background(mut self, brush: impl Into<PaintBrush>) -> Self {
        self.background = Some(brush.into());
        self
    }

    /// Paint a border around the widget with a color or a gradient.
    pub fn border(mut self, brush: impl Into<PaintBrush>, width: f64) -> Self {
        self.border = Some(BorderStyle {
            width,
            brush: brush.into(),
        });
        self
    }

    /// Round off corners of this container by setting a corner radius
    pub fn rounded(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    #[cfg(test)]
    pub(crate) fn background_is_some(&self) -> bool {
        self.background.is_some()
    }

    #[cfg(test)]
    pub(crate) fn border_is_some(&self) -> bool {
        self.border.is_some()
    }
}

impl<T: Data> Widget<T> for Container<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Container");

        // Shrink constraints by border offset
        let border_width = match self.border {
            Some(ref border) => border.width,
            None => 0.0,
        };
        let child_bc = bc.shrink((2.0 * border_width, 2.0 * border_width));
        let size = self.inner.layout(ctx, &child_bc, data, env);
        let origin = Point::new(border_width, border_width);
        self.inner
            .set_layout_rect(Rect::from_origin_size(origin, size));

        let my_size = Size::new(
            size.width + 2.0 * border_width,
            size.height + 2.0 * border_width,
        );

        let my_insets = self.inner.compute_parent_paint_rect(my_size);
        ctx.set_paint_insets(my_insets);
        my_size
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        let panel = RoundedRect::from_origin_size(
            Point::ORIGIN,
            paint_ctx.size().to_vec2(),
            self.corner_radius,
        );

        if let Some(border) = &self.border {
            paint_ctx.stroke(panel, &border.brush, border.width);
        };

        if let Some(background) = &self.background {
            paint_ctx.fill(panel, background);
        };

        self.inner.paint(paint_ctx, data, env);
    }
}
