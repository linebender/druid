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

//! A convenience widget that combines common styling and positioning widgets.

use crate::shell::kurbo::{Affine, Point, Rect, Size};
use crate::shell::piet::{PaintBrush, RenderContext};
use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

struct BorderState {
    width: f64,
    brush: PaintBrush,
}

#[derive(Default)]
struct ContainerStyle {
    background: Option<PaintBrush>,
    border: Option<BorderState>,
}

/// A convenience widget that combines common styling and positioning widgets.
pub struct Container<T: Data> {
    style: ContainerStyle,
    inner: Box<dyn Widget<T>>,
}

impl<T: Data + 'static> Container<T> {
    /// Create Container with a child
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            style: ContainerStyle::default(),
            inner: Box::new(inner),
        }
    }

    /// Paint background with a color or a gradient.
    pub fn background(mut self, brush: impl Into<PaintBrush>) -> Self {
        self.style.background = Some(brush.into());
        self
    }

    /// Paint a border around the widget with a color or a gradient.
    pub fn border(mut self, brush: impl Into<PaintBrush>, width: f64) -> Self {
        self.style.border = Some(BorderState {
            width,
            brush: brush.into(),
        });
        self
    }
}

impl<T: Data + 'static> Widget<T> for Container<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        // Add border offset
        if let Some(ref border) = self.style.border {
            let offset = border.width / 2.0;
            paint_ctx.transform(Affine::translate((offset, offset)));
        }

        // Paint background color
        if let Some(ref brush) = self.style.background {
            let rect = Rect::from_origin_size(Point::ZERO, base_state.size());
            paint_ctx.render_ctx.fill(rect, brush);
        }

        // Paint border
        if let Some(ref border) = self.style.border {
            let rect = Rect::from_origin_size((0.0, 0.0), base_state.size());
            paint_ctx
                .render_ctx
                .stroke(rect, &border.brush, border.width);
        }

        // Paint child
        self.inner.paint(paint_ctx, base_state, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        // Shrink constraints by border offset
        let border_width = match self.style.border {
            Some(ref border) => border.width,
            None => 0.0,
        };
        let child_bc = bc.shrink((border_width, border_width));
        self.inner.layout(ctx, &child_bc, data, env)
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut T, env: &Env) {
        self.inner.event(event, ctx, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }
}
