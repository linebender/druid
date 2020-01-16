// Copyright 2018 The xi-editor Authors.
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

use crate::kurbo::{Insets, Point, Rect, Size};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget, WidgetId,
    WidgetPod,
};

/// A widget that just adds padding around its child.
pub struct Padding<T: Data> {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,

    child: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Padding<T> {
    /// Create widget with uniform padding.
    #[deprecated(since = "0.3.0", note = "Use Padding::new() instead")]
    pub fn uniform(padding: f64, child: impl Widget<T> + 'static) -> Padding<T> {
        Padding {
            left: padding,
            right: padding,
            top: padding,
            bottom: padding,
            child: WidgetPod::new(child).boxed(),
        }
    }

    /// Create a new widget with the specified padding. This can either be an instance
    /// of [`kurbo::Insets`], a f64 for uniform padding, a 2-tuple for axis-uniform padding
    /// or 4-tuple with (left, top, right, bottom) values.
    ///
    /// # Examples
    ///
    /// Uniform padding:
    ///
    /// ```
    /// use druid::widget::{Label, Padding};
    /// use druid::kurbo::Insets;
    ///
    /// let _: Padding<()> = Padding::new(10.0, Label::new("uniform!"));
    /// let _: Padding<()> = Padding::new(Insets::uniform(10.0), Label::new("uniform!"));
    /// ```
    ///
    /// Uniform padding across each axis:
    ///
    /// ```
    /// use druid::widget::{Label, Padding};
    /// use druid::kurbo::Insets;
    ///
    /// let child: Label<()> = Label::new("I need my space!");
    /// let _: Padding<()> = Padding::new((10.0, 20.0), Label::new("more y than x!"));
    /// // equivalent:
    /// let _: Padding<()> = Padding::new(Insets::uniform_xy(10.0, 20.0), Label::new("ditto :)"));
    /// ```
    ///
    /// [`kurbo::Insets`]: https://docs.rs/kurbo/0.5.3/kurbo/struct.Insets.html
    pub fn new(insets: impl Into<Insets>, child: impl Widget<T> + 'static) -> Padding<T> {
        let insets = insets.into();
        Padding {
            left: insets.x0,
            right: insets.x1,
            top: insets.y0,
            bottom: insets.y1,
            child: WidgetPod::new(child).boxed(),
        }
    }
}

impl<T: Data> Widget<T> for Padding<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Padding");

        let hpad = self.left + self.right;
        let vpad = self.top + self.bottom;

        let child_bc = bc.shrink((hpad, vpad));
        let size = self.child.layout(layout_ctx, &child_bc, data, env);
        let origin = Point::new(self.left, self.top);
        self.child
            .set_layout_rect(Rect::from_origin_size(origin, size));
        Size::new(size.width + hpad, size.height + vpad)
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint_with_offset(paint_ctx, data, env);
    }

    fn get_id(&self) -> Option<WidgetId> {
        Some(self.child.id())
    }
}
