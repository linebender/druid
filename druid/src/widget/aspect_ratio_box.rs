// Copyright 2021 The Druid Authors.
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
    inner: Box<dyn Widget<T>>,
    ratio: f64,
}

impl<T> AspectRatioBox<T> {
    /// Create container with a child and aspect ratio.
    ///
    /// The aspect ratio is defined as width / height.
    ///
    /// If aspect ratio <= 0.0, the ratio will be set to 1.0
    pub fn new(inner: impl Widget<T> + 'static, ratio: f64) -> Self {
        Self {
            inner: Box::new(inner),
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
        self.inner.event(ctx, event, data, env);
    }

    #[instrument(
        name = "AspectRatioBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    #[instrument(
        name = "AspectRatioBox",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
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

            return self.inner.layout(ctx, &bc, data, env);
        }

        if bc.max().width == f64::INFINITY && bc.max().height == f64::INFINITY {
            warn!("Box constraints are INFINITE. Aspect ratio box won't be able to choose a size because the constraints given by the parent widget are INFINITE.");

            return self.inner.layout(ctx, &bc, data, env);
        }

        let bc = self.generate_constraints(bc);

        self.inner.layout(ctx, &bc, data, env)
    }

    #[instrument(name = "AspectRatioBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
