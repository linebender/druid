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

//! A widget that provides simple visual styling options to a child.

use super::BackgroundBrush;
use crate::shell::kurbo::{Point, Rect, RoundedRect, Size};
use crate::{
    BoxConstraints, Color, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, UpdateCtx, Widget, WidgetPod,
};

struct BorderStyle {
    width: KeyOrValue<f64>,
    color: KeyOrValue<Color>,
}

/// A widget that provides simple visual styling options to a child.
pub struct Container<T> {
    background: Option<BackgroundBrush<T>>,
    border: Option<BorderStyle>,
    corner_radius: f64,

    inner: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Container<T> {
    /// Create Container with a child
    pub fn new(inner: impl Widget<T> + 'static) -> Self {
        Self {
            background: None,
            border: None,
            corner_radius: 0.0,
            inner: WidgetPod::new(inner).boxed(),
        }
    }

    /// Set the background for this widget.
    ///
    /// This can be passed anything which can be represented by a [`BackgroundBrush`];
    /// noteably, it can be any [`Color`], a [`Key<Color>`] resolvable in the [`Env`],
    /// any gradient, or a fully custom [`Painter`] widget.
    ///
    /// [`BackgroundBrush`]: ../enum.BackgroundBrush.html
    /// [`Color`]: ../struct.Color.thml
    /// [`Key<Color>`]: ../struct.Key.thml
    /// [`Env`]: ../struct.Env.html
    /// [`Painter`]: struct.Painter.html
    pub fn background(mut self, brush: impl Into<BackgroundBrush<T>>) -> Self {
        self.background = Some(brush.into());
        self
    }

    /// Paint a border around the widget with a color and width.
    ///
    /// Arguments can be either concrete values, or a [`Key`] of the respective
    /// type.
    ///
    /// [`Key`]: struct.Key.html
    pub fn border(
        mut self,
        color: impl Into<KeyOrValue<Color>>,
        width: impl Into<KeyOrValue<f64>>,
    ) -> Self {
        self.border = Some(BorderStyle {
            color: color.into(),
            width: width.into(),
        });
        self
    }

    /// Round off corners of this container by setting a corner radius
    pub fn rounded(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    #[cfg(test)]
    pub(crate) fn background_is_some(&self) -> bool {
        self.background.is_some()
    }

    #[cfg(test)]
    pub(crate) fn border_is_some(&self) -> bool {
        self.border.is_some()
    }
}

impl<T: Data> Widget<T> for Container<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Container");

        // Shrink constraints by border offset
        let border_width = match self.border {
            Some(ref border) => border.width.resolve(env),
            None => 0.0,
        };
        let child_bc = bc.shrink((2.0 * border_width, 2.0 * border_width));
        let size = self.inner.layout(ctx, &child_bc, data, env);
        let origin = Point::new(border_width, border_width);
        self.inner
            .set_layout_rect(Rect::from_origin_size(origin, size));

        let my_size = Size::new(
            size.width + 2.0 * border_width,
            size.height + 2.0 * border_width,
        );

        let my_insets = self.inner.compute_parent_paint_insets(my_size);
        ctx.set_paint_insets(my_insets);
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let panel =
            RoundedRect::from_origin_size(Point::ORIGIN, ctx.size().to_vec2(), self.corner_radius);

        if let Err(e) = ctx.save() {
            log::error!("{}", e);
            return;
        }

        ctx.clip(panel);

        if let Some(background) = self.background.as_mut() {
            background.paint(ctx, data, env);
        }

        if let Err(e) = ctx.restore() {
            log::error!("{}", e);
        }

        if let Some(border) = &self.border {
            ctx.stroke(panel, &border.color.resolve(env), border.width.resolve(env));
        };

        self.inner.paint(ctx, data, env);
    }
}
