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
use crate::{BoxConstraints, HandlerCtx, Id, LayoutCtx, LayoutResult, PaintCtx, Ui};

use crate::kurbo::{Rect, Size};
use crate::piet::{Color, FillRule, RenderContext};

const BOX_HEIGHT: f64 = 24.;
const BACKGROUND_COLOR: Color = Color::rgb24(0x55_55_55);
const BAR_COLOR: Color = Color::rgb24(0xf0_f0_ea);

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
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Rect) {
        //Paint the background
        let brush = paint_ctx.render_ctx.solid_brush(BACKGROUND_COLOR);

        paint_ctx.render_ctx.fill(geom, &brush, FillRule::NonZero);

        //Paint the bar
        let brush = paint_ctx.render_ctx.solid_brush(BAR_COLOR);

        let calculated_bar_width = self.value * geom.width() as f64;

        let rect = geom.with_size(Size::new(calculated_bar_width, geom.height()));
        paint_ctx.render_ctx.fill(rect, &brush, FillRule::NonZero);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<Size>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain(Size::new(bc.max.width, BOX_HEIGHT as f64)))
    }

    fn poke(&mut self, payload: &mut dyn Any, ctx: &mut HandlerCtx) -> bool {
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
