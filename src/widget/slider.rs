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

//! A slider widget.

use crate::widget::Widget;
use crate::{
    BoxConstraints, Geometry, HandlerCtx, Id, LayoutCtx, LayoutResult, MouseEvent, PaintCtx, Ui,
};

use crate::kurbo::Rect;
use crate::piet::{Color, FillRule, RenderContext};

const BOX_HEIGHT: f64 = 24.;
const BACKGROUND_COLOR: Color = Color::rgb24(0x55_55_55);
const SLIDER_COLOR: Color = Color::rgb24(0xf0_f0_ea);

pub struct Slider {
    value: f64,
}

impl Slider {
    pub fn new(initial_value: f64) -> Slider {
        Slider {
            value: initial_value,
        }
    }

    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for Slider {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        //Paint the background
        let brush = paint_ctx.render_ctx.solid_brush(BACKGROUND_COLOR);

        let (x, y) = geom.pos;
        let (width, height) = geom.size;
        let rect = Rect::new(
            x as f64,
            y as f64,
            x as f64 + width as f64,
            y as f64 + height as f64,
        );

        paint_ctx.render_ctx.fill(rect, &brush, FillRule::NonZero);

        //Paint the slider
        let brush = paint_ctx.render_ctx.solid_brush(SLIDER_COLOR);

        let (width, height) = geom.size;
        let (width, height) = (width as f64, height as f64);
        let (x, y) = geom.pos;
        let (x, y) = (x as f64, y as f64);

        let slider_absolute_position = (width - BOX_HEIGHT) * self.value + BOX_HEIGHT / 2.;
        let half_box = height / 2.;
        let full_box = height;

        let mut calculated_position = slider_absolute_position - half_box;
        if calculated_position < 0. {
            calculated_position = 0.;
        } else if (calculated_position + full_box) > width {
            calculated_position = width - full_box;
        }

        let rect = Rect::new(
            x + calculated_position,
            y,
            x + calculated_position + full_box,
            y + height,
        );

        paint_ctx.render_ctx.fill(rect, &brush, FillRule::NonZero);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((bc.max_width, BOX_HEIGHT as f32)))
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count == 1 {
            ctx.set_active(true);
            self.value = ((event.x as f64 - BOX_HEIGHT / 2.)
                / (ctx.get_geom().size.0 as f64 - BOX_HEIGHT))
                .max(0.0)
                .min(1.0);
            ctx.send_event(self.value);
        } else {
            ctx.set_active(false);
        }
        ctx.invalidate();
        true
    }

    fn mouse_moved(&mut self, x: f32, _y: f32, ctx: &mut HandlerCtx) {
        if ctx.is_active() {
            self.value = ((x as f64 - BOX_HEIGHT / 2.)
                / (ctx.get_geom().size.0 as f64 - BOX_HEIGHT))
                .max(0.0)
                .min(1.0);

            ctx.send_event(self.value);
            ctx.invalidate();
        }
    }
}
