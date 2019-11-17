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

use crate::kurbo::{Point, RoundedRect, Size};
use crate::piet::{LinearGradient, RenderContext, UnitPoint};
use crate::theme;
use crate::widget::Align;
use crate::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

/// A progress bar, displaying a numeric progress value.
#[derive(Debug, Clone, Default)]
pub struct ProgressBar {}

impl ProgressBar {
    pub fn new() -> impl Widget<f64> {
        Align::vertical(UnitPoint::CENTER, Self::default())
    }
}

impl Widget<f64> for ProgressBar {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &f64, env: &Env) {
        let clamped = data.max(0.0).min(1.0);

        let rounded_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            (Size {
                width: base_state.size().width,
                height: env.get(theme::BASIC_WIDGET_HEIGHT),
            })
            .to_vec2(),
            4.,
        );

        //Paint the border
        paint_ctx.stroke(rounded_rect, &env.get(theme::BORDER), 2.0);

        //Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );
        paint_ctx.fill(rounded_rect, &background_gradient);

        //Paint the bar
        let calculated_bar_width = clamped * rounded_rect.width();
        let rounded_rect = RoundedRect::from_origin_size(
            Point::ORIGIN,
            (Size {
                width: calculated_bar_width,
                height: env.get(theme::BASIC_WIDGET_HEIGHT),
            })
            .to_vec2(),
            4.,
        );
        let bar_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (env.get(theme::PRIMARY_LIGHT), env.get(theme::PRIMARY_DARK)),
        );
        paint_ctx.fill(rounded_rect, &bar_gradient);
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        env: &Env,
    ) -> Size {
        bc.debug_check("ProgressBar");

        let default_width = 100.0;

        if bc.is_width_bounded() {
            bc.constrain(Size::new(
                bc.max().width,
                env.get(theme::BASIC_WIDGET_HEIGHT),
            ))
        } else {
            bc.constrain(Size::new(
                default_width,
                env.get(theme::BASIC_WIDGET_HEIGHT),
            ))
        }
    }

    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut f64, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&f64>, _data: &f64, _env: &Env) {
        ctx.invalidate();
    }
}
