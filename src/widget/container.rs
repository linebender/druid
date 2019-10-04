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

use crate::shell::kurbo::{Point, Rect, Size};
use crate::shell::piet::{PaintBrush, RenderContext, StrokeStyle};
use crate::widget::Padding;
use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use std::marker::PhantomData;

struct BorderState {
    width: f64,
    brush: PaintBrush,
    style: StrokeStyle,
}

#[derive(Default)]
struct ContainerStyle {
    color: Option<PaintBrush>,
    border: Option<BorderState>,
}

// TODO: add description
pub struct Container<T: Data> {
    padding: f64,
    style: ContainerStyle,
    phantom: PhantomData<T>,
}

impl<T: Data + 'static> Container<T> {
    pub fn new() -> Self {
        Self {
            padding: 0.0,
            style: ContainerStyle::default(),
            phantom: PhantomData::default(),
        }
    }

    pub fn child(self, inner: impl Widget<T> + 'static) -> impl Widget<T> {
        Padding::uniform(self.raw_padding(), ContainerRaw::new(self.style, inner))
    }

    pub fn empty(self) -> impl Widget<T> {
        Padding::uniform(self.raw_padding(), ContainerRaw::empty(self.style))
    }

    pub fn color(mut self, brush: impl Into<PaintBrush>) -> Self {
        self.style.color = Some(brush.into());
        self
    }

    pub fn border(mut self, brush: impl Into<PaintBrush>, width: f64) -> Self {
        self.style.border = Some(BorderState {
            width,
            brush: brush.into(),
            style: StrokeStyle::new(),
        });
        self
    }

    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    fn raw_padding(&self) -> f64 {
        let border_padding = match self.style.border {
            Some(ref border) => border.width / 2.0,
            None => 0.0,
        };

        self.padding + border_padding
    }
}

struct ContainerRaw<T: Data> {
    style: ContainerStyle,
    inner: Option<Box<dyn Widget<T>>>,
}

impl<T: Data + 'static> ContainerRaw<T> {
    fn new(style: ContainerStyle, inner: impl Widget<T> + 'static) -> Self {
        Self {
            style,
            inner: Some(Box::new(inner)),
        }
    }

    fn empty(style: ContainerStyle) -> Self {
        Self { style, inner: None }
    }
}

impl<T: Data> Widget<T> for ContainerRaw<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &T, env: &Env) {
        // Paint background color
        if let Some(ref brush) = self.style.color {
            let rect = Rect::from_origin_size(Point::ZERO, base_state.size());
            paint_ctx.render_ctx.fill(rect, brush);
        }

        // Paint border
        if let Some(ref border) = self.style.border {
            let rect = Rect::from_origin_size((0.0, 0.0), base_state.size());
            paint_ctx
                .render_ctx
                .stroke_styled(rect, &border.brush, border.width, &border.style);
        }

        // Paint child
        if let Some(ref mut inner) = self.inner {
            inner.paint(paint_ctx, base_state, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if let Some(ref mut inner) = self.inner {
            inner.layout(ctx, bc, data, env)
        } else {
            Size::ZERO
        }
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.event(event, ctx, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.update(ctx, old_data, data, env);
        }
    }
}
