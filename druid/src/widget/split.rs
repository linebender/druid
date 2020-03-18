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

//! A widget which splits an area in two, with a settable ratio, and optional draggable resizing.

use crate::kurbo::{Line, Point, Rect, Size};
use crate::widget::flex::Axis;
use crate::{
    theme, BoxConstraints, Color, Cursor, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, UpdateCtx, Widget, WidgetPod,
};

///A container containing two other widgets, splitting the area either horizontally or vertically.
pub struct Split<T> {
    split_direction: Axis,
    solid: bool,
    draggable: bool,
    min_size: f64,
    split_point: f64,
    splitter_size: f64,
    child1: WidgetPod<T, Box<dyn Widget<T>>>,
    child2: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T> Split<T> {
    ///Create a new split panel.
    fn new(
        split_direction: Axis,
        child1: impl Widget<T> + 'static,
        child2: impl Widget<T> + 'static,
    ) -> Self {
        Split {
            split_direction,
            min_size: 0.0,
            solid: false,
            split_point: 0.5,
            splitter_size: 10.0,
            draggable: false,
            child1: WidgetPod::new(child1).boxed(),
            child2: WidgetPod::new(child2).boxed(),
        }
    }
    /// Create a new split panel, with a vertical splitter between two children.
    pub fn vertical(child1: impl Widget<T> + 'static, child2: impl Widget<T> + 'static) -> Self {
        Self::new(Axis::Vertical, child1, child2)
    }
    /// Create a new split panel, with a horizontal splitter between two children.
    pub fn horizontal(child1: impl Widget<T> + 'static, child2: impl Widget<T> + 'static) -> Self {
        Self::new(Axis::Horizontal, child1, child2)
    }
    /// Set container's split point as a fraction of the split dimension
    /// The value must be between 0.0 and 1.0, exclusive
    pub fn split_point(mut self, split_point: f64) -> Self {
        assert!(
            split_point > 0.0 && split_point < 1.0,
            "split_point must be between 0.0 and 1.0!"
        );
        self.split_point = split_point;
        self
    }
    /// Builder-style method to set the minimum size for both sides of the split.
    ///
    /// The value must be greater than or equal to `0.0`.
    pub fn min_size(mut self, min_size: f64) -> Self {
        assert!(min_size >= 0.0);
        self.min_size = min_size;

        self
    }
    /// Set the width of the splitter bar, in pixels
    /// The value must be positive or zero
    pub fn splitter_size(mut self, splitter_size: f64) -> Self {
        assert!(
            splitter_size >= 0.0,
            "splitter_width must be 0.0 or greater!"
        );
        self.splitter_size = splitter_size;
        self
    }
    /// Set whether the splitter's split point can be changed by dragging.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }
    /// Builder-style method to set whether the splitter handle is drawn as a solid rectangle.
    ///
    /// If this is `false` (the default), it will be drawn as two parallel lines.
    pub fn fill_splitter_handle(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }
    fn splitter_hit_test(&self, size: Size, mouse_pos: Point) -> bool {
        match self.split_direction {
            Axis::Vertical => {
                let center = size.width * self.split_point;
                (center - mouse_pos.x).abs() < self.splitter_size.min(5.0) / 2.0
            }
            Axis::Horizontal => {
                let center = size.height * self.split_point;
                (center - mouse_pos.y).abs() < self.splitter_size.min(5.0) / 2.0
            }
        }
    }
    fn calculate_limits(&self, size: Size) -> (f64, f64) {
        // Since the Axis::Direction tells us the direction of the splitter itself
        // we need the minor axis to get the size of the 'splitted' direction
        let size_in_splitted_direction = self.split_direction.minor(size);

        let min_offset = (self.splitter_size * 0.5).min(5.0);
        let mut min_limit = self.min_size.max(min_offset);
        let mut max_limit = (size_in_splitted_direction - self.min_size.max(min_offset)).max(0.0);

        if min_limit > max_limit {
            min_limit = 0.5 * (min_limit + max_limit);
            max_limit = min_limit;
        }

        (min_limit, max_limit)
    }

    fn update_splitter(&mut self, size: Size, mouse_pos: Point) {
        self.split_point = match self.split_direction {
            Axis::Vertical => {
                let (min_limit, max_limit) = self.calculate_limits(size);
                clamp(mouse_pos.x, min_limit, max_limit) / size.width
            }
            Axis::Horizontal => {
                let (min_limit, max_limit) = self.calculate_limits(size);
                clamp(mouse_pos.y, min_limit, max_limit) / size.height
            }
        }
    }
    fn get_edges(&mut self, ctx: &PaintCtx) -> (f64, f64) {
        let size = ctx.size();
        match self.split_direction {
            Axis::Vertical => {
                let reduced_width = size.width - self.splitter_size;
                let edge1 = reduced_width * self.split_point;
                let edge2 = edge1 + self.splitter_size;
                (edge1, edge2)
            }
            Axis::Horizontal => {
                let reduced_height = size.height - self.splitter_size;
                let edge1 = reduced_height * self.split_point;
                let edge2 = edge1 + self.splitter_size;
                (edge1, edge2)
            }
        }
    }
    fn get_color(&self, env: &Env) -> Color {
        if self.draggable {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        }
    }
    fn paint_solid(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let size = ctx.size();
        //third, because we're putting the lines at roughly third points.
        //small, because we floor, to give the extra pixel (roughly) to the middle.
        let small_third = (self.splitter_size / 3.0).floor();
        let (edge1, edge2) = self.get_edges(ctx);
        let rect = match self.split_direction {
            Axis::Vertical => Rect::from_points(
                Point::new(edge1 + small_third, 0.0),
                Point::new(edge2 - small_third, size.height),
            ),
            Axis::Horizontal => Rect::from_points(
                Point::new(0.0, edge1 + small_third),
                Point::new(size.width, edge2 - small_third),
            ),
        };
        let splitter_color = self.get_color(env);
        ctx.fill(rect, &splitter_color);
    }
    fn paint_stroked(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let size = ctx.size();
        //third, because we're putting the lines at roughly third points.
        //small, because we floor, to give the extra pixel (roughly) to the middle.
        let small_third = (self.splitter_size / 3.0).floor();
        let (edge1, edge2) = self.get_edges(ctx);
        let (line1, line2) = match self.split_direction {
            Axis::Vertical => (
                Line::new(
                    Point::new(edge1 + small_third, 0.0),
                    Point::new(edge1 + small_third, size.height),
                ),
                Line::new(
                    Point::new(edge2 - small_third, 0.0),
                    Point::new(edge2 - small_third, size.height),
                ),
            ),
            Axis::Horizontal => (
                Line::new(
                    Point::new(0.0, edge1 + small_third),
                    Point::new(size.width, edge1 + small_third),
                ),
                Line::new(
                    Point::new(0.0, edge2 - small_third),
                    Point::new(size.width, edge2 - small_third),
                ),
            ),
        };
        let splitter_color = self.get_color(env);
        ctx.stroke(line1, &splitter_color, 1.0);
        ctx.stroke(line2, &splitter_color, 1.0);
    }
}
impl<T: Data> Widget<T> for Split<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.child1.is_active() {
            self.child1.event(ctx, event, data, env);
            if ctx.is_handled() {
                return;
            }
        }
        if self.child2.is_active() {
            self.child2.event(ctx, event, data, env);
            if ctx.is_handled() {
                return;
            }
        }
        if self.draggable {
            match event {
                Event::MouseDown(mouse) => {
                    if mouse.button.is_left() && self.splitter_hit_test(ctx.size(), mouse.pos) {
                        ctx.set_active(true);
                        ctx.set_handled();
                    }
                }
                Event::MouseUp(mouse) => {
                    if mouse.button.is_left() && ctx.is_active() {
                        ctx.set_active(false);
                        self.update_splitter(ctx.size(), mouse.pos);
                        ctx.request_paint();
                    }
                }
                Event::MouseMoved(mouse) => {
                    if ctx.is_active() {
                        self.update_splitter(ctx.size(), mouse.pos);
                        ctx.request_layout();
                    }

                    if ctx.is_hot() && self.splitter_hit_test(ctx.size(), mouse.pos)
                        || ctx.is_active()
                    {
                        match self.split_direction {
                            Axis::Horizontal => ctx.set_cursor(&Cursor::ResizeUpDown),
                            Axis::Vertical => ctx.set_cursor(&Cursor::ResizeLeftRight),
                        };
                    }
                }
                _ => {}
            }
        }
        if !self.child1.is_active() {
            self.child1.event(ctx, event, data, env);
        }
        if !self.child2.is_active() {
            self.child2.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child1.lifecycle(ctx, event, data, env);
        self.child2.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child1.update(ctx, &data, env);
        self.child2.update(ctx, &data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Split");

        let mut my_size = bc.max();

        let reduced_width = my_size.width - self.splitter_size;
        let reduced_height = my_size.height - self.splitter_size;
        let (child1_bc, child2_bc) = match self.split_direction {
            Axis::Vertical => {
                if !bc.is_width_bounded() {
                    log::warn!("A Split widget was given an unbounded width to split.")
                }
                let child1_width = (reduced_width * self.split_point).max(0.0);
                let child2_width = (reduced_width - child1_width).max(0.0);
                (
                    BoxConstraints::new(
                        Size::new(child1_width, bc.min().height),
                        Size::new(child1_width, bc.max().height),
                    ),
                    BoxConstraints::new(
                        Size::new(child2_width, bc.min().height),
                        Size::new(child2_width, bc.max().height),
                    ),
                )
            }
            Axis::Horizontal => {
                if !bc.is_width_bounded() {
                    log::warn!("A Split widget was given an unbounded height to split.")
                }
                let child1_height = (reduced_height * self.split_point).max(0.0);
                let child2_height = (reduced_height - child1_height).max(0.0);
                (
                    BoxConstraints::new(
                        Size::new(bc.min().width, child1_height),
                        Size::new(bc.max().width, child1_height),
                    ),
                    BoxConstraints::new(
                        Size::new(bc.min().width, child2_height),
                        Size::new(bc.max().width, child2_height),
                    ),
                )
            }
        };
        let child1_size = self.child1.layout(ctx, &child1_bc, &data, env);
        let child2_size = self.child2.layout(ctx, &child2_bc, &data, env);

        //Top-left align for both children, out of laziness.
        //Reduce our unsplit direction to the larger of the two widgets
        let (child1_rect, child2_rect) = match self.split_direction {
            Axis::Vertical => {
                my_size.height = child1_size.height.max(child2_size.height);
                (
                    Rect::from_origin_size(Point::ORIGIN, child1_size),
                    Rect::from_origin_size(
                        Point::new(child1_size.width + self.splitter_size, 0.0),
                        child2_size,
                    ),
                )
            }
            Axis::Horizontal => {
                my_size.width = child1_size.width.max(child2_size.width);
                (
                    Rect::from_origin_size(Point::ORIGIN, child1_size),
                    Rect::from_origin_size(
                        Point::new(0.0, child1_size.height + self.splitter_size),
                        child2_size,
                    ),
                )
            }
        };
        self.child1.set_layout_rect(child1_rect);
        self.child2.set_layout_rect(child2_rect);

        let paint_rect = self.child1.paint_rect().union(self.child2.paint_rect());
        let insets = paint_rect - Rect::ZERO.with_size(my_size);
        ctx.set_paint_insets(insets);

        // Update our splits to hold our constraints if needed
        let (min_limit, max_limit) = self.calculate_limits(my_size);
        self.split_point = match self.split_direction {
            Axis::Vertical => {
                if my_size.width <= std::f64::EPSILON {
                    0.5
                } else {
                    clamp(
                        self.split_point,
                        min_limit / my_size.width,
                        max_limit / my_size.width,
                    )
                }
            }
            Axis::Horizontal => {
                if my_size.height <= std::f64::EPSILON {
                    0.5
                } else {
                    clamp(
                        self.split_point,
                        min_limit / my_size.height,
                        max_limit / my_size.height,
                    )
                }
            }
        };

        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.solid {
            self.paint_solid(ctx, env);
        } else {
            self.paint_stroked(ctx, env);
        }
        self.child1.paint_with_offset(ctx, &data, env);
        self.child2.paint_with_offset(ctx, &data, env);
    }
}

// Move to std lib clamp as soon as https://github.com/rust-lang/rust/issues/44095 lands
fn clamp(mut x: f64, min: f64, max: f64) -> f64 {
    assert!(min <= max);
    if x < min {
        x = min;
    }
    if x > max {
        x = max;
    }
    x
}
