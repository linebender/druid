// Copyright 2019 The Druid Authors.
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
use crate::theme;
use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, LinearGradient,
    PaintCtx, RenderContext, UnitPoint, UpdateCtx, Widget,
};

/// A progress bar, displaying a numeric progress value.
///
/// This type impls `Widget<f64>`, expecting a float in the range `0.0..1.0`.
#[derive(Debug, Clone, Default)]
pub struct ProgressBar;

impl ProgressBar {
    /// Return a new `ProgressBar`.
    pub fn new() -> ProgressBar {
        Self::default()
    }
}

impl Widget<f64> for ProgressBar {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut f64, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &f64, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &f64, _data: &f64, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &f64,
        env: &Env,
    ) -> Size {
        bc.debug_check("ProgressBar");
        bc.constrain(Size::new(
            env.get(theme::WIDE_WIDGET_WIDTH),
            env.get(theme::BASIC_WIDGET_HEIGHT),
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, env: &Env) {
        let height = env.get(theme::BASIC_WIDGET_HEIGHT);
        let corner_radius = env.get(theme::PROGRESS_BAR_RADIUS);
        let clamped = data.max(0.0).min(1.0);
        let stroke_width = 2.0;
        let inset = -stroke_width / 2.0;
        let size = ctx.size();
        let rounded_rect = Size::new(size.width, height)
            .to_rect()
            .inset(inset)
            .to_rounded_rect(corner_radius);

        // Paint the border
        ctx.stroke(rounded_rect, &env.get(theme::BORDER_DARK), stroke_width);

        // Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );
        ctx.fill(rounded_rect, &background_gradient);

        // Paint the bar
        let calculated_bar_width = clamped * rounded_rect.width();

        let rounded_rect = Rect::from_origin_size(
            Point::new(-inset, 0.),
            Size::new(calculated_bar_width, height),
        )
        .inset((0.0, inset))
        .to_rounded_rect(corner_radius);

        let bar_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (env.get(theme::PRIMARY_LIGHT), env.get(theme::PRIMARY_DARK)),
        );
        ctx.fill(rounded_rect, &bar_gradient);
    }
}
