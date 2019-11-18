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

use crate::shell::kurbo::{Point, Rect, Size};
use crate::shell::piet::{PaintBrush, RenderContext};
use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
    WidgetPod,
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
    inner: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Container<T> {
    /// Create Container with a child
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            style: ContainerStyle::default(),
            inner: WidgetPod::new(inner).boxed(),
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
        // Paint background color
        if let Some(ref brush) = self.style.background {
            let rect = Rect::from_origin_size(Point::ZERO, base_state.size());
            paint_ctx.render_ctx.fill(rect, brush);
        }

        // Paint border
        if let Some(ref border) = self.style.border {
            let offset = border.width / 2.0;
            let size = Size::new(
                base_state.size().width - border.width,
                base_state.size().height - border.width,
            );
            let rect = Rect::from_origin_size((offset, offset), size);
            paint_ctx
                .render_ctx
                .stroke(rect, &border.brush, border.width);
        }

        // Paint child
        self.inner.paint_with_offset(paint_ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Container");

        // Shrink constraints by border offset
        let border_width = match self.style.border {
            Some(ref border) => border.width,
            None => 0.0,
        };
        let child_bc = bc.shrink((2.0 * border_width, 2.0 * border_width));
        let size = self.inner.layout(ctx, &child_bc, data, env);
        let origin = Point::new(border_width, border_width);
        self.inner
            .set_layout_rect(Rect::from_origin_size(origin, size));

        Size::new(
            size.width + 2.0 * border_width,
            size.height + 2.0 * border_width,
        )
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
    }
}
