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

//! A progress bar widget.

use std::any::Any;

use crate::widget::Widget;
use crate::{
    BoxConstraints, Geometry, HandlerCtx, Id, LayoutCtx, LayoutResult, MouseEvent, PaintCtx, Ui,
};

use kurbo::{Line, Rect};
use piet::{FillRule, RenderContext};

const BOX_HEIGHT: f64 = 24.;

pub struct ProgressBar {
    value: f64,
}

impl ProgressBar {
    pub fn new(initial_value: f64) -> ProgressBar {
        ProgressBar {
            value: initial_value,
        }
    }
    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for ProgressBar {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {

        let background_color = 0x55_55_55_ff;
        let bar_color = 0xf0f0eaff;

        //Paint the background
        let brush = paint_ctx.render_ctx.solid_brush(background_color).unwrap();

        let (x, y) = geom.pos;
        let (width, height) = geom.size;
        let rect = Rect::new(
            x as f64,
            y as f64,
            x as f64 + width as f64,
            y as f64 + height as f64,
        );

        paint_ctx.render_ctx.fill(rect, &brush, FillRule::NonZero);

        //Paint the bar 
        let brush = paint_ctx.render_ctx.solid_brush(bar_color).unwrap();

        let (width, height) = geom.size;
        let (x, y) = geom.pos;

        let calculated_bar_width = self.value * width as f64;

        let rect = Rect::new(
            x as f64,
            y as f64,
            x as f64 + calculated_bar_width,
            y as f64 + height as f64,
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

    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        if let Some(value) = payload.downcast_ref::<f64>() {
            self.value = *value;
            ctx.invalidate();
            true
        } else {
            println!("downcast failed");
            false
        }
    }

}
