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

//! A widget that just adds padding during layout.

use crate::debug_state::DebugState;
use crate::widget::{prelude::*, WidgetWrapper};
use crate::{Data, Insets, KeyOrValue, Point, WidgetPod};

use tracing::{instrument, trace};

/// A widget that just adds padding around its child.
pub struct Padding<T, W> {
    insets: KeyOrValue<Insets>,
    child: WidgetPod<T, W>,
}

impl<T, W: Widget<T>> Padding<T, W> {
    /// Create a new `Padding` with the specified padding and child.
    ///
    /// The `insets` argument can either be an instance of [`Insets`],
    /// a [`Key`] referring to [`Insets`] in the [`Env`],
    /// an `f64` for uniform padding, an `(f64, f64)` for axis-uniform padding,
    /// or `(f64, f64, f64, f64)` (left, top, right, bottom) values.
    ///
    /// # Examples
    ///
    /// Uniform padding:
    ///
    /// ```
    /// use druid::widget::{Label, Padding};
    /// use druid::kurbo::Insets;
    ///
    /// let _: Padding<(), _> = Padding::new(10.0, Label::new("uniform!"));
    /// let _: Padding<(), _> = Padding::new(Insets::uniform(10.0), Label::new("uniform!"));
    /// ```
    ///
    /// Uniform padding across each axis:
    ///
    /// ```
    /// use druid::widget::{Label, Padding};
    /// use druid::kurbo::Insets;
    ///
    /// let child: Label<()> = Label::new("I need my space!");
    /// let _: Padding<(), _> = Padding::new((10.0, 20.0), Label::new("more y than x!"));
    /// // equivalent:
    /// let _: Padding<(), _> = Padding::new(Insets::uniform_xy(10.0, 20.0), Label::new("ditto :)"));
    /// ```
    ///
    /// [`Key`]: crate::Key
    pub fn new(insets: impl Into<KeyOrValue<Insets>>, child: W) -> Padding<T, W> {
        Padding {
            insets: insets.into(),
            child: WidgetPod::new(child),
        }
    }
}

impl<T, W> WidgetWrapper for Padding<T, W> {
    widget_wrapper_pod_body!(W, child);
}

impl<T: Data, W: Widget<T>> Widget<T> for Padding<T, W> {
    #[instrument(name = "Padding", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env)
    }

    #[instrument(name = "Padding", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    #[instrument(name = "Padding", level = "trace", skip(self, ctx, _old, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old: &T, data: &T, env: &Env) {
        if ctx.env_key_changed(&self.insets) {
            ctx.request_layout();
        }
        self.child.update(ctx, data, env);
    }

    #[instrument(name = "Padding", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Padding");
        let insets = self.insets.resolve(env);

        let hpad = insets.x0 + insets.x1;
        let vpad = insets.y0 + insets.y1;

        let child_bc = bc.shrink((hpad, vpad));
        let size = self.child.layout(ctx, &child_bc, data, env);
        let origin = Point::new(insets.x0, insets.y0);
        self.child.set_origin(ctx, data, env, origin);

        let my_size = Size::new(size.width + hpad, size.height + vpad);
        let my_insets = self.child.compute_parent_paint_insets(my_size);
        ctx.set_paint_insets(my_insets);
        let baseline_offset = self.child.baseline_offset();
        if baseline_offset > 0f64 {
            ctx.set_baseline_offset(baseline_offset + insets.y1);
        }
        trace!("Computed layout: size={}, insets={:?}", my_size, my_insets);
        my_size
    }

    #[instrument(name = "Padding", level = "trace", skip(self, ctx, data, env))]
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
