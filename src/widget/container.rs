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

// TODO: update description
//! A convenience widget that combines common painting, positioning, and sizing widgets.

use crate::shell::kurbo::{Point, Rect, Size, Affine};
use crate::shell::piet::{Color, PaintBrush, RenderContext, StrokeStyle};
use crate::widget::{Padding, SizedBox};
use crate::{
    Action, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx,
    Widget,
};

struct BorderState {
    width: f64,
    brush: PaintBrush,
    // TODO: do we need this?
    style: StrokeStyle,
}

// TODO: add description
pub struct Container<T: Data> {
    inner: Option<Box<dyn Widget<T>>>,
    color: Option<PaintBrush>,
    // TODO: add border state for each side
    border: Option<BorderState>,
}

impl<T: Data + 'static> Container<T> {
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        let mut container = Self::empty();
        container.inner = Some(Box::new(inner));
        container
    }

    pub fn empty() -> Self {
        Self {
            inner: None,
            color: None,
            border: None,
        }
    }

    pub fn color(mut self, brush: impl Into<PaintBrush>) -> Self {
        self.color = Some(brush.into());
        self
    }

    pub fn border(mut self, brush: impl Into<PaintBrush>, width: f64) -> Self {
        self.border = Some(BorderState {
            width,
            brush: brush.into(),
            style: StrokeStyle::new(),
        });
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        match self.inner {
            Some(inner) => {
                self.inner = Some(Box::new(Padding::uniform(padding, inner)));
            }
            None => {
                self.inner = Some(Box::new(Padding::uniform(padding, SizedBox::empty())));
            }
        }
        self
    }
}

impl<T: Data> Widget<T> for Container<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        // Paint background color
        if let Some(ref brush) = self.color {
            let rect = Rect::from_origin_size(Point::ZERO, base_state.size());
            paint_ctx.render_ctx.fill(rect, brush);
        }

        // Paint border
        if let Some(ref border) = self.border {
            // Shift border rect by half of the border width. This is needed so that border
            // doesn't paint outside of the given constraints.
            let origin = (border.width/2.0, border.width/2.0);
            let mut size = base_state.size();
            let rect = Rect::from_origin_size(origin, size);
            paint_ctx
                .render_ctx
                .stroke_styled(rect, &border.brush, border.width, &border.style);

            dbg!("border", rect);
            dbg!(base_state.size());

            // Move child to be inside the border.
            paint_ctx.render_ctx.transform(Affine::translate((border.width/2.0, border.width/2.0)));
        }

        // Paint child
        if let Some(ref mut inner) = self.inner {
            inner.paint(paint_ctx, base_state, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if let Some(ref mut inner) = self.inner {
            let mut child_bc = bc.clone();
            if let Some(ref border) = self.border {
                // If container is with border then we need to decrease the space available
                // for the child.

                // TODO: is there a better way to write this?
                child_bc.max.width -= border.width;
                child_bc.max.height -= border.width;
                child_bc.min.width = child_bc.min.width.min(child_bc.max.width);
                child_bc.min.height = child_bc.min.height.min(child_bc.max.height);
                println!("decrease bc");
                dbg!(bc);
                dbg!(child_bc);
            }

            inner.layout(ctx, &child_bc, data, env)
        } else {
            Size::ZERO
        }
    }

    fn event(
        &mut self,
        event: &Event,
        ctx: &mut EventCtx,
        data: &mut T,
        env: &Env,
    ) -> Option<Action> {
        if let Some(ref mut inner) = self.inner {
            inner.event(event, ctx, data, env)
        } else {
            None
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.update(ctx, old_data, data, env);
        }
    }
}
