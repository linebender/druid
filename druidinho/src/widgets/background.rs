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

//! A widget that provides simple visual styling options to a child.

use super::layout::LayoutHost;
use crate::kurbo::{Point, Size};
use crate::piet::{Color, RenderContext};
use crate::widget::WidgetHolder;
use crate::{BoxConstraints, LayoutCtx, PaintCtx, Widget};

struct BorderStyle {
    width: f64,
    color: Color,
}

/// A widget that provides simple visual styling options to a child.
pub struct Background<W> {
    background: Option<Color>,
    border: Option<BorderStyle>,
    corner_radius: f64,
    inner: LayoutHost<W>,
}

impl<W> Background<W> {
    /// Create Container with a child
    pub fn new(inner: W) -> Self {
        Self {
            background: None,
            border: None,
            corner_radius: 0.0.into(),
            inner: LayoutHost::new(inner),
        }
    }

    /// Builder-style method for setting the background for this widget.
    pub fn background(mut self, brush: Color) -> Self {
        self.background = Some(brush);
        self
    }

    /// Builder-style method for setting a border.
    pub fn border(mut self, color: Color, width: f64) -> Self {
        self.border = Some(BorderStyle { color, width });
        self
    }

    /// Builder style method for rounding off corners of this container by setting a corner radius
    pub fn rounded(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }
}

impl<W: Widget> WidgetHolder for Background<W> {
    type Child = LayoutHost<W>;

    fn widget(&self) -> &Self::Child {
        &self.inner
    }

    fn widget_mut(&mut self) -> &mut Self::Child {
        &mut self.inner
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        bc.debug_check("Container");
        let border_width = self.border.as_ref().map(|b| b.width).unwrap_or(0.0);
        let child_bc = bc.shrink((2.0 * border_width, 2.0 * border_width));
        let size = WidgetHolder::layout(&mut self.inner, ctx, child_bc);
        let origin = Point::new(border_width, border_width);
        self.inner.set_origin(origin);

        let my_size = Size::new(
            size.width + 2.0 * border_width,
            size.height + 2.0 * border_width,
        );

        my_size
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let size = self.inner.size();
        if let Some(color) = self.background.as_ref() {
            let panel = size.to_rounded_rect(self.corner_radius);
            ctx.fill(panel, color);
        }

        if let Some(border) = &self.border {
            let border_rect = size
                .to_rect()
                .inset(border.width / -2.0)
                .to_rounded_rect(self.corner_radius);
            ctx.stroke(border_rect, &border.color, border.width);
        };

        WidgetHolder::paint(&self.inner, ctx);
    }
}
