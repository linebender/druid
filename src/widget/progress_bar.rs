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

use crate::kurbo::{Point, Rect, Size};
use crate::piet::{Color, FillRule, RenderContext};
use crate::{
    Action, BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

const BACKGROUND_COLOR: Color = Color::rgb24(0x55_55_55);
const BAR_COLOR: Color = Color::rgb24(0xf0_f0_ea);

/// A progress bar, displaying a numeric progress value.
#[derive(Debug, Clone, Default)]
pub struct ProgressBar {}

impl Widget<f64> for ProgressBar {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &f64, _env: &Env) {
        let clamped = data.max(0.0).min(1.0);
        let rect = Rect::from_origin_size(Point::ORIGIN, base_state.size());

        //Paint the background
        let brush = paint_ctx.solid_brush(BACKGROUND_COLOR);
        paint_ctx.fill(rect, &brush, FillRule::NonZero);

        //Paint the bar
        let brush = paint_ctx.solid_brush(BAR_COLOR);
        let calculated_bar_width = clamped * rect.width();
        let rect = rect.with_size(Size::new(calculated_bar_width, rect.height()));
        paint_ctx.fill(rect, &brush, FillRule::NonZero);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        _env: &Env,
    ) -> Size {
        bc.constrain(bc.max())
    }

    fn event(
        &mut self,
        _event: &Event,
        _ctx: &mut EventCtx,
        _data: &mut f64,
        _env: &Env,
    ) -> Option<Action> {
        None
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&f64>, _data: &f64, _env: &Env) {
        ctx.invalidate();
    }
}
