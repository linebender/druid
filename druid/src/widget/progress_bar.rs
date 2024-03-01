// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A progress bar widget.

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::{theme, LinearGradient, Point, Rect, UnitPoint};
use tracing::instrument;

/// A progress bar, displaying a numeric progress value.
///
/// This type impls `Widget<f64>`, expecting a float in the range `0.0..1.0`.
#[derive(Debug, Clone, Default)]
pub struct ProgressBar;

impl ProgressBar {
    /// Return a new `ProgressBar`.
    pub fn new() -> ProgressBar {
        Self
    }
}

impl Widget<f64> for ProgressBar {
    #[instrument(
        name = "ProgressBar",
        level = "trace",
        skip(self, _ctx, _event, _data, _env)
    )]
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut f64, _env: &Env) {}

    #[instrument(
        name = "ProgressBar",
        level = "trace",
        skip(self, _ctx, _event, _data, _env)
    )]
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &f64, _env: &Env) {}

    #[instrument(
        name = "ProgressBar",
        level = "trace",
        skip(self, ctx, _old_data, _data, _env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &f64, _data: &f64, _env: &Env) {
        ctx.request_paint();
    }

    #[instrument(
        name = "ProgressBar",
        level = "trace",
        skip(self, _layout_ctx, bc, _data, env)
    )]
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

    #[instrument(name = "ProgressBar", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &f64, env: &Env) {
        let height = env.get(theme::BASIC_WIDGET_HEIGHT);
        let corner_radius = env.get(theme::PROGRESS_BAR_RADIUS);
        let clamped = data.clamp(0.0, 1.0);
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

    fn debug_state(&self, data: &f64) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: data.to_string(),
            ..Default::default()
        }
    }
}
