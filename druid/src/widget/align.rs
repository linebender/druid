// Copyright 2018 The Druid Authors.
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

//! A widget that aligns its child (for example, centering it).

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::{Data, Rect, Size, UnitPoint, WidgetPod};
use tracing::{instrument, trace};
use crate::contexts::CommandCtx;

/// A widget that aligns its child.
pub struct Align<T> {
    align: UnitPoint,
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
    in_viewport: bool,
    viewport: Rect,
}

impl<T> Align<T> {
    /// Create widget with alignment.
    ///
    /// Note that the `align` parameter is specified as a `UnitPoint` in
    /// terms of left and right. This is inadequate for bidi-aware layout
    /// and thus the API will change when druid gains bidi capability.
    pub fn new(align: UnitPoint, child: impl Widget<T> + 'static) -> Align<T> {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
            width_factor: None,
            height_factor: None,
            in_viewport: false,
            viewport: Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY),
        }
    }

    /// Create centered widget.
    pub fn centered(child: impl Widget<T> + 'static) -> Align<T> {
        Align::new(UnitPoint::CENTER, child)
    }

    /// Create right-aligned widget.
    pub fn right(child: impl Widget<T> + 'static) -> Align<T> {
        Align::new(UnitPoint::RIGHT, child)
    }

    /// Create left-aligned widget.
    pub fn left(child: impl Widget<T> + 'static) -> Align<T> {
        Align::new(UnitPoint::LEFT, child)
    }

    /// Align only in the horizontal axis, keeping the child's size in the vertical.
    pub fn horizontal(align: UnitPoint, child: impl Widget<T> + 'static) -> Align<T> {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
            width_factor: None,
            height_factor: Some(1.0),
            in_viewport: false,
            viewport: Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY),
        }
    }

    /// Align only in the vertical axis, keeping the child's size in the horizontal.
    pub fn vertical(align: UnitPoint, child: impl Widget<T> + 'static) -> Align<T> {
        Align {
            align,
            child: WidgetPod::new(child).boxed(),
            width_factor: Some(1.0),
            height_factor: None,
            in_viewport: false,
            viewport: Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY),
        }
    }

    fn in_viewport(mut self) -> Self {
        self.in_viewport = true;
        self
    }

    fn align<'b, C: CommandCtx<'b>>(&mut self, ctx: &mut C, data: &T, env: &Env, my_size: Size) {
        let size = self.child.layout_rect().size();

        let extra_width = (my_size.width - size.width).max(0.);
        let extra_height = (my_size.height - size.height).max(0.);

        // The part of our layout_rect the origin of the child is allowed to be in
        let mut extra_space = Rect::new(0., 0., extra_width, extra_height);

        if self.in_viewport {
            // The part of the viewport the origin of the child is allowed to be in
            let viewport = Rect::from_origin_size(self.viewport.origin(), self.viewport.size() - size);

            // Essentially Rect::intersect but this implementation chooses the point closed to viewport
            // inside extra_space to give the child a valid origin even if this widget is not inside
            // the viewport
            extra_space.x0 = extra_space.x0.max(viewport.x0).min(extra_space.x1);
            extra_space.y0 = extra_space.y0.max(viewport.y0).min(extra_space.y1);
            extra_space.x1 = extra_space.x1.min(viewport.x1).max(extra_space.x0);
            extra_space.y1 = extra_space.y1.min(viewport.y1).max(extra_space.y0);
        }

        let origin = self
            .align
            .resolve(extra_space)
            .expand();
        self.child.set_origin(ctx, data, env, origin);
    }
}

impl<T: Data> Widget<T> for Align<T> {
    #[instrument(name = "Align", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env)
    }

    #[instrument(name = "Align", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::ViewContextChanged(view_ctx) = event {
            self.viewport = view_ctx.clip;
            if self.in_viewport {
                self.align(ctx, data, env, ctx.size());
            }
        }

        self.child.lifecycle(ctx, event, data, env)
    }

    #[instrument(name = "Align", level = "trace", skip(self, ctx, _old_data, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    #[instrument(name = "Align", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        trace!("Layout constraints: {:?}", bc);
        bc.debug_check("Align");

        let size = self.child.layout(ctx, &bc.loosen(), data, env);

        log_size_warnings(size);

        let mut my_size = size;
        if bc.is_width_bounded() {
            my_size.width = bc.max().width;
        }
        if bc.is_height_bounded() {
            my_size.height = bc.max().height;
        }

        if let Some(width) = self.width_factor {
            my_size.width = size.width * width;
        }
        if let Some(height) = self.height_factor {
            my_size.height = size.height * height;
        }

        let my_size = bc.constrain(my_size);
        self.align(ctx, data, env, my_size);

        let my_insets = self.child.compute_parent_paint_insets(my_size);
        ctx.set_paint_insets(my_insets);

        if self.height_factor.is_some() {
            let baseline_offset = self.child.baseline_offset();
            if baseline_offset > 0f64 {
                ctx.set_baseline_offset(my_size.height - self.child.layout_rect().y1 + baseline_offset);
            }
        }

        trace!(
            "Computed layout: origin={}, size={}, insets={:?}",
            self.child.layout_rect().origin(),
            my_size,
            my_insets
        );
        my_size
    }

    #[instrument(name = "Align", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

fn log_size_warnings(size: Size) {
    if size.width.is_infinite() {
        tracing::warn!("Align widget's child has an infinite width.");
    }

    if size.height.is_infinite() {
        tracing::warn!("Align widget's child has an infinite height.");
    }
}
