// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::debug_state::DebugState;

use crate::widget::Axis;
use druid::widget::prelude::*;
use druid::Data;
use tracing::{instrument, warn};

/// A widget that preserves the aspect ratio given to it.
///
/// If given a child, this widget forces the child to have a width and height that preserves
/// the aspect ratio.
///
/// If not given a child, The box will try to size itself  as large or small as possible
/// to preserve the aspect ratio.
pub struct AspectRatioBox<T> {
    child: Box<dyn Widget<T>>,
    ratio: f64,
}

impl<T> AspectRatioBox<T> {
    /// Create container with a child and aspect ratio.
    ///
    /// The aspect ratio is defined as width / height.
    ///
    /// If aspect ratio <= 0.0, the ratio will be set to 1.0
    pub fn new(child: impl Widget<T> + 'static, ratio: f64) -> Self {
        Self {
            child: Box::new(child),
            ratio: clamp_ratio(ratio),
        }
    }

    /// Set the ratio of the box.
    ///
    /// The ratio has to be a value between 0 and f64::MAX, excluding 0. It will be clamped
    /// to those values if they exceed the bounds. If the ratio is 0, then the ratio
    /// will become 1.
    pub fn set_ratio(&mut self, ratio: f64) {
        self.ratio = clamp_ratio(ratio);
    }

    /// Generate `BoxConstraints` that fit within the provided `BoxConstraints`.
    ///
    /// If the generated constraints do not fit then they are constrained to the
    /// provided `BoxConstraints`.
    fn generate_constraints(&self, bc: &BoxConstraints) -> BoxConstraints {
        let (mut new_width, mut new_height) = (bc.max().width, bc.max().height);

        if new_width == f64::INFINITY {
            new_width = new_height * self.ratio;
        } else {
            new_height = new_width / self.ratio;
        }

        if new_width > bc.max().width {
            new_width = bc.max().width;
            new_height = new_width / self.ratio;
        }

        if new_height > bc.max().height {
            new_height = bc.max().height;
            new_width = new_height * self.ratio;
        }

        if new_width < bc.min().width {
            new_width = bc.min().width;
            new_height = new_width / self.ratio;
        }

        if new_height < bc.min().height {
            new_height = bc.min().height;
            new_width = new_height * self.ratio;
        }

        BoxConstraints::tight(bc.constrain(Size::new(new_width, new_height)))
    }
}

/// Clamps the ratio between 0.0 and f64::MAX
/// If ratio is 0.0 then it will return 1.0 to avoid creating NaN
fn clamp_ratio(mut ratio: f64) -> f64 {
    ratio = f64::clamp(ratio, 0.0, f64::MAX);

    if ratio == 0.0 {
        warn!("Provided ratio was <= 0.0.");
        1.0
    } else {
        ratio
    }
}

impl<T: Data> Widget<T> for AspectRatioBox<T> {
    #[instrument(
        name = "AspectRatioBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    #[instrument(
        name = "AspectRatioBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    #[instrument(
        name = "AspectRatioBox",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    #[instrument(
        name = "AspectRatioBox",
        level = "trace",
        skip(self, ctx, bc, data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("AspectRatioBox");

        if bc.max() == bc.min() {
            warn!("Box constraints are tight. Aspect ratio box will not be able to preserve aspect ratio.");

            return self.child.layout(ctx, bc, data, env);
        }
        if bc.max().width == f64::INFINITY && bc.max().height == f64::INFINITY {
            warn!("Box constraints are INFINITE. Aspect ratio box won't be able to choose a size because the constraints given by the parent widget are INFINITE.");

            return self.child.layout(ctx, bc, data, env);
        }

        let bc = self.generate_constraints(bc);

        self.child.layout(ctx, &bc, data, env)
    }

    #[instrument(name = "AspectRatioBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        self.child.id()
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.debug_state(data)],
            ..Default::default()
        }
    }

    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        match axis {
            Axis::Horizontal => {
                if bc.is_height_bounded() {
                    bc.max().height * self.ratio
                } else {
                    self.child.compute_max_intrinsic(axis, ctx, bc, data, env)
                }
            }
            Axis::Vertical => {
                if bc.is_width_bounded() {
                    bc.max().width / self.ratio
                } else {
                    self.child.compute_max_intrinsic(axis, ctx, bc, data, env)
                }
            }
        }
    }
}
