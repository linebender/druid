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
use crate::{BoxConstraints, HandlerCtx, Id, LayoutCtx, LayoutResult, MouseEvent, PaintCtx, Ui};

use crate::kurbo::{Point, Rect, Size};
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
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        //Paint the background
        let brush = paint_ctx.render_ctx.solid_brush(BACKGROUND_COLOR);

        paint_ctx.render_ctx.fill(geom, &brush, FillRule::NonZero);
        //Paint the slider
        let brush = paint_ctx.render_ctx.solid_brush(SLIDER_COLOR);

        let slider_absolute_position = (geom.width() - BOX_HEIGHT) * self.value + BOX_HEIGHT / 2.;
        let half_box = geom.height() / 2.;
        let full_box = geom.height();

        let mut position = slider_absolute_position - half_box;
        if position < 0. {
            position = 0.;
        } else if (position + full_box) > geom.width() {
            position = geom.width() - full_box;
        }

        let knob_orig = Point::new(geom.origin().x + position, geom.origin().y);
        let knob_size = Size::new(full_box, geom.height());
        let knob_rect = Rect::from_origin_size(knob_orig, knob_size);

        paint_ctx
            .render_ctx
            .fill(knob_rect, &brush, FillRule::NonZero);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<Size>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain(Size::new(bc.max.width, BOX_HEIGHT)))
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count == 1 {
            ctx.set_active(true);
            self.value = ((event.pos.x - BOX_HEIGHT / 2.) / (ctx.get_geom().width() - BOX_HEIGHT))
                .max(0.0)
                .min(1.0);
            ctx.send_event(self.value);
        } else {
            ctx.set_active(false);
        }
        ctx.invalidate();
        true
    }

    fn mouse_moved(&mut self, pos: Point, ctx: &mut HandlerCtx) {
        if ctx.is_active() {
            self.value = ((pos.x - BOX_HEIGHT / 2.) / (ctx.get_geom().width() - BOX_HEIGHT))
                .max(0.0)
                .min(1.0);

            ctx.send_event(self.value);
            ctx.invalidate();
        }
    }
}
