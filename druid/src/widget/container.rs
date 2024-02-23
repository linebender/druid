// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that provides simple visual styling options to a child.

use super::BackgroundBrush;
use crate::debug_state::DebugState;
use crate::kurbo::RoundedRectRadii;
use crate::widget::prelude::*;
use crate::widget::Axis;
use crate::{Color, Data, KeyOrValue, Point, WidgetPod};
use tracing::{instrument, trace, trace_span};

struct BorderStyle {
    width: KeyOrValue<f64>,
    color: KeyOrValue<Color>,
}

/// A widget that provides simple visual styling options to a child.
pub struct Container<T> {
    background: Option<BackgroundBrush<T>>,
    foreground: Option<BackgroundBrush<T>>,
    border: Option<BorderStyle>,
    corner_radius: KeyOrValue<RoundedRectRadii>,

    child: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Container<T> {
    /// Create Container with a child
    pub fn new(child: impl Widget<T> + 'static) -> Self {
        Self {
            background: None,
            foreground: None,
            border: None,
            corner_radius: 0.0.into(),
            child: WidgetPod::new(child).boxed(),
        }
    }

    /// Builder-style method for setting the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// notably, it can be any [`Color`], a [`Key<Color>`] resolvable in the [`Env`],
    /// any gradient, or a fully custom [`Painter`] widget.
    ///
    /// [`Key<Color>`]: crate::Key
    /// [`Painter`]: super::Painter
    pub fn background(mut self, brush: impl Into<BackgroundBrush<T>>) -> Self {
        self.set_background(brush);
        self
    }

    /// Set the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// notably, it can be any [`Color`], a [`Key<Color>`] resolvable in the [`Env`],
    /// any gradient, or a fully custom [`Painter`] widget.
    ///
    /// [`Key<Color>`]: crate::Key
    /// [`Painter`]: super::Painter
    pub fn set_background(&mut self, brush: impl Into<BackgroundBrush<T>>) {
        self.background = Some(brush.into());
    }

    /// Clears background.
    pub fn clear_background(&mut self) {
        self.background = None;
    }

    /// Builder-style method for setting the foreground for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// notably, it can be any [`Color`], a [`Key<Color>`] resolvable in the [`Env`],
    /// any gradient, or a fully custom [`Painter`] widget.
    ///
    /// [`Key<Color>`]: crate::Key
    /// [`Painter`]: super::Painter
    pub fn foreground(mut self, brush: impl Into<BackgroundBrush<T>>) -> Self {
        self.set_foreground(brush);
        self
    }

    /// Set the foreground for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// notably, it can be any [`Color`], a [`Key<Color>`] resolvable in the [`Env`],
    /// any gradient, or a fully custom [`Painter`] widget.
    ///
    /// [`Key<Color>`]: crate::Key
    /// [`Painter`]: super::Painter
    pub fn set_foreground(&mut self, brush: impl Into<BackgroundBrush<T>>) {
        self.foreground = Some(brush.into());
    }

    /// Clears foreground.
    pub fn clear_foreground(&mut self) {
        self.foreground = None;
    }

    /// Builder-style method for painting a border around the widget with a color and width.
    ///
    /// Arguments can be either concrete values, or a [`Key`] of the respective
    /// type.
    ///
    /// [`Key`]: crate::Key
    pub fn border(
        mut self,
        color: impl Into<KeyOrValue<Color>>,
        width: impl Into<KeyOrValue<f64>>,
    ) -> Self {
        self.set_border(color, width);
        self
    }

    /// Paint a border around the widget with a color and width.
    ///
    /// Arguments can be either concrete values, or a [`Key`] of the respective
    /// type.
    ///
    /// [`Key`]: crate::Key
    pub fn set_border(
        &mut self,
        color: impl Into<KeyOrValue<Color>>,
        width: impl Into<KeyOrValue<f64>>,
    ) {
        self.border = Some(BorderStyle {
            color: color.into(),
            width: width.into(),
        });
    }

    /// Clears border.
    pub fn clear_border(&mut self) {
        self.border = None;
    }

    /// Builder style method for rounding off corners of this container by setting a corner radius
    pub fn rounded(mut self, radius: impl Into<KeyOrValue<RoundedRectRadii>>) -> Self {
        self.set_rounded(radius);
        self
    }

    /// Round off corners of this container by setting a corner radius
    pub fn set_rounded(&mut self, radius: impl Into<KeyOrValue<RoundedRectRadii>>) {
        self.corner_radius = radius.into();
    }

    #[cfg(test)]
    pub(crate) fn background_is_some(&self) -> bool {
        self.background.is_some()
    }

    #[cfg(test)]
    pub(crate) fn foreground_is_some(&self) -> bool {
        self.foreground.is_some()
    }

    #[cfg(test)]
    pub(crate) fn border_is_some(&self) -> bool {
        self.border.is_some()
    }
}

impl<T: Data> Widget<T> for Container<T> {
    #[instrument(name = "Container", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    #[instrument(name = "Container", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    #[instrument(
        name = "Container",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(brush) = self.background.as_mut() {
            trace_span!("update background").in_scope(|| {
                brush.update(ctx, old_data, data, env);
            });
        }
        if let Some(brush) = self.foreground.as_mut() {
            trace_span!("update foreground").in_scope(|| {
                brush.update(ctx, old_data, data, env);
            });
        }
        if let Some(border) = &self.border {
            if ctx.env_key_changed(&border.width) {
                ctx.request_layout();
            }
            if ctx.env_key_changed(&border.color) {
                ctx.request_paint();
            }
        }
        if ctx.env_key_changed(&self.corner_radius) {
            ctx.request_paint();
        }
        self.child.update(ctx, data, env);
    }

    #[instrument(name = "Container", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Container");

        // Shrink constraints by border offset
        let border_width = match &self.border {
            Some(border) => border.width.resolve(env),
            None => 0.0,
        };
        let child_bc = bc.shrink((2.0 * border_width, 2.0 * border_width));
        let size = self.child.layout(ctx, &child_bc, data, env);
        let origin = Point::new(border_width, border_width);
        self.child.set_origin(ctx, origin);

        let my_size = Size::new(
            size.width + 2.0 * border_width,
            size.height + 2.0 * border_width,
        );

        let my_insets = self.child.compute_parent_paint_insets(my_size);
        ctx.set_paint_insets(my_insets);
        let baseline_offset = self.child.baseline_offset();
        if baseline_offset > 0f64 {
            ctx.set_baseline_offset(baseline_offset + border_width);
        }

        trace!("Computed layout: size={}, insets={:?}", my_size, my_insets);
        my_size
    }

    #[instrument(name = "Container", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let corner_radius = self.corner_radius.resolve(env);

        if let Some(background) = self.background.as_mut() {
            let panel = ctx.size().to_rounded_rect(corner_radius);

            trace_span!("paint background").in_scope(|| {
                ctx.with_save(|ctx| {
                    ctx.clip(panel);
                    background.paint(ctx, data, env);
                });
            });
        }

        if let Some(border) = &self.border {
            let border_width = border.width.resolve(env);
            let border_rect = ctx
                .size()
                .to_rect()
                .inset(border_width / -2.0)
                .to_rounded_rect(corner_radius);
            ctx.stroke(border_rect, &border.color.resolve(env), border_width);
        };

        self.child.paint(ctx, data, env);

        if let Some(foreground) = self.foreground.as_mut() {
            let panel = ctx.size().to_rounded_rect(corner_radius);

            trace_span!("paint foreground").in_scope(|| {
                ctx.with_save(|ctx| {
                    ctx.clip(panel);
                    foreground.paint(ctx, data, env);
                });
            });
        }
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.widget().debug_state(data)],
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
        let container_width = match &self.border {
            Some(border) => border.width.resolve(env),
            None => 0.0,
        };
        let child_bc = bc.shrink((2.0 * container_width, 2.0 * container_width));
        let child_size = self
            .child
            .widget_mut()
            .compute_max_intrinsic(axis, ctx, &child_bc, data, env);
        let border_width_on_both_sides = container_width * 2.;
        child_size + border_width_on_both_sides
    }
}
