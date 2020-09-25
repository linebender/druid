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

//! A widget which splits an area in two, with a settable ratio, and optional draggable resizing.

use crate::kurbo::{Line, Point, Rect, Size};
use crate::widget::flex::Axis;
use crate::{
    theme, BoxConstraints, Color, Cursor, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, RenderContext, UpdateCtx, Widget, WidgetPod,
};

/// A container containing two other widgets, splitting the area either horizontally or vertically.
pub struct Split<T> {
    split_axis: Axis,
    split_point_chosen: f64,
    split_point_effective: f64,
    min_size: f64,     // Integers only
    bar_size: f64,     // Integers only
    min_bar_area: f64, // Integers only
    solid: bool,
    draggable: bool,
    child1: WidgetPod<T, Box<dyn Widget<T>>>,
    child2: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T> Split<T> {
    /// Create a new split panel, with the specified axis being split in two.
    ///
    /// Horizontal split axis means that the children are left and right.
    /// Vertical split axis means that the children are up and down.
    fn new(
        split_axis: Axis,
        child1: impl Widget<T> + 'static,
        child2: impl Widget<T> + 'static,
    ) -> Self {
        Split {
            split_axis,
            split_point_chosen: 0.5,
            split_point_effective: 0.5,
            min_size: 0.0,
            bar_size: 6.0,
            min_bar_area: 6.0,
            solid: false,
            draggable: false,
            child1: WidgetPod::new(child1).boxed(),
            child2: WidgetPod::new(child2).boxed(),
        }
    }

    /// Create a new split panel, with the horizontal axis split in two by a vertical bar.
    /// The children are laid out left and right.
    pub fn columns(child1: impl Widget<T> + 'static, child2: impl Widget<T> + 'static) -> Self {
        Self::new(Axis::Horizontal, child1, child2)
    }

    /// Create a new split panel, with the vertical axis split in two by a horizontal bar.
    /// The children are laid out up and down.
    pub fn rows(child1: impl Widget<T> + 'static, child2: impl Widget<T> + 'static) -> Self {
        Self::new(Axis::Vertical, child1, child2)
    }

    /// Builder-style method to set the split point as a fraction of the split axis.
    ///
    /// The value must be between `0.0` and `1.0`, inclusive.
    /// The default split point is `0.5`.
    pub fn split_point(mut self, split_point: f64) -> Self {
        assert!(
            split_point >= 0.0 && split_point <= 1.0,
            "split_point must be in the range [0.0-1.0]!"
        );
        self.split_point_chosen = split_point;
        self
    }

    /// Builder-style method to set the minimum size for both sides of the split axis.
    ///
    /// The value must be greater than or equal to `0.0`.
    /// The value will be rounded up to the nearest integer.
    pub fn min_size(mut self, min_size: f64) -> Self {
        assert!(min_size >= 0.0);
        self.min_size = min_size.ceil();
        self
    }

    /// Builder-style method to set the size of the splitter bar.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default splitter bar size is `6.0`.
    pub fn bar_size(mut self, bar_size: f64) -> Self {
        assert!(bar_size >= 0.0, "bar_size must be 0.0 or greater!");
        self.bar_size = bar_size.ceil();
        self
    }

    /// Builder-style method to set the minimum size of the splitter bar area.
    ///
    /// The minimum splitter bar area defines the minimum size of the area
    /// where mouse hit detection is done for the splitter bar.
    /// The final area is either this or the splitter bar size, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter bar,
    /// but don't want to sacrifice user experience by making it hard to click on.
    ///
    /// The value must be positive or zero.
    /// The value will be rounded up to the nearest integer.
    /// The default minimum splitter bar area is `6.0`.
    pub fn min_bar_area(mut self, min_bar_area: f64) -> Self {
        assert!(min_bar_area >= 0.0, "min_bar_area must be 0.0 or greater!");
        self.min_bar_area = min_bar_area.ceil();
        self
    }

    /// Builder-style method to set whether the split point can be changed by dragging.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Builder-style method to set whether the splitter bar is drawn as a solid rectangle.
    ///
    /// If this is `false` (the default), the bar will be drawn as two parallel lines.
    pub fn solid_bar(mut self, solid: bool) -> Self {
        self.solid = solid;
        self
    }

    /// Returns the size of the splitter bar area.
    #[inline]
    fn bar_area(&self) -> f64 {
        self.bar_size.max(self.min_bar_area)
    }

    /// Returns the padding size added to each side of the splitter bar.
    #[inline]
    fn bar_padding(&self) -> f64 {
        (self.bar_area() - self.bar_size) / 2.0
    }

    /// Returns the location of the edges of the splitter bar area,
    /// given the specified total size.
    fn bar_edges(&self, size: Size) -> (f64, f64) {
        let bar_area = self.bar_area();
        match self.split_axis {
            Axis::Horizontal => {
                let reduced_width = size.width - bar_area;
                let edge1 = (reduced_width * self.split_point_effective).floor();
                let edge2 = edge1 + bar_area;
                (edge1, edge2)
            }
            Axis::Vertical => {
                let reduced_height = size.height - bar_area;
                let edge1 = (reduced_height * self.split_point_effective).floor();
                let edge2 = edge1 + bar_area;
                (edge1, edge2)
            }
        }
    }

    /// Returns true if the provided mouse position is inside the splitter bar area.
    fn bar_hit_test(&self, size: Size, mouse_pos: Point) -> bool {
        let (edge1, edge2) = self.bar_edges(size);
        match self.split_axis {
            Axis::Horizontal => mouse_pos.x >= edge1 && mouse_pos.x <= edge2,
            Axis::Vertical => mouse_pos.y >= edge1 && mouse_pos.y <= edge2,
        }
    }

    /// Returns the minimum and maximum split coordinate of the provided size.
    fn split_side_limits(&self, size: Size) -> (f64, f64) {
        let split_axis_size = self.split_axis.major(size);

        let mut min_limit = self.min_size;
        let mut max_limit = (split_axis_size - min_limit).max(0.0);

        if min_limit > max_limit {
            min_limit = 0.5 * (min_limit + max_limit);
            max_limit = min_limit;
        }

        (min_limit, max_limit)
    }

    /// Set a new chosen split point.
    fn update_split_point(&mut self, size: Size, mouse_pos: Point) {
        let (min_limit, max_limit) = self.split_side_limits(size);
        self.split_point_chosen = match self.split_axis {
            Axis::Horizontal => clamp(mouse_pos.x, min_limit, max_limit) / size.width,
            Axis::Vertical => clamp(mouse_pos.y, min_limit, max_limit) / size.height,
        }
    }

    /// Returns the color of the splitter bar.
    fn bar_color(&self, env: &Env) -> Color {
        if self.draggable {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        }
    }

    fn paint_solid_bar(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let size = ctx.size();
        let (edge1, edge2) = self.bar_edges(size);
        let padding = self.bar_padding();
        let rect = match self.split_axis {
            Axis::Horizontal => Rect::from_points(
                Point::new(edge1 + padding.ceil(), 0.0),
                Point::new(edge2 - padding.floor(), size.height),
            ),
            Axis::Vertical => Rect::from_points(
                Point::new(0.0, edge1 + padding.ceil()),
                Point::new(size.width, edge2 - padding.floor()),
            ),
        };
        let splitter_color = self.bar_color(env);
        ctx.fill(rect, &splitter_color);
    }

    fn paint_stroked_bar(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let size = ctx.size();
        // Set the line width to a third of the splitter bar size,
        // because we'll paint two equal lines at the edges.
        let line_width = (self.bar_size / 3.0).floor();
        let line_midpoint = line_width / 2.0;
        let (edge1, edge2) = self.bar_edges(size);
        let padding = self.bar_padding();
        let (line1, line2) = match self.split_axis {
            Axis::Horizontal => (
                Line::new(
                    Point::new(edge1 + line_midpoint + padding.ceil(), 0.0),
                    Point::new(edge1 + line_midpoint + padding.ceil(), size.height),
                ),
                Line::new(
                    Point::new(edge2 - line_midpoint - padding.floor(), 0.0),
                    Point::new(edge2 - line_midpoint - padding.floor(), size.height),
                ),
            ),
            Axis::Vertical => (
                Line::new(
                    Point::new(0.0, edge1 + line_midpoint + padding.ceil()),
                    Point::new(size.width, edge1 + line_midpoint + padding.ceil()),
                ),
                Line::new(
                    Point::new(0.0, edge2 - line_midpoint - padding.floor()),
                    Point::new(size.width, edge2 - line_midpoint - padding.floor()),
                ),
            ),
        };
        let splitter_color = self.bar_color(env);
        ctx.stroke(line1, &splitter_color, line_width);
        ctx.stroke(line2, &splitter_color, line_width);
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
                    if mouse.button.is_left() && self.bar_hit_test(ctx.size(), mouse.pos) {
                        ctx.set_active(true);
                        ctx.set_handled();
                    }
                }
                Event::MouseUp(mouse) => {
                    if mouse.button.is_left() && ctx.is_active() {
                        ctx.set_active(false);
                        self.update_split_point(ctx.size(), mouse.pos);
                        ctx.request_paint();
                    }
                }
                Event::MouseMove(mouse) => {
                    if ctx.is_active() {
                        self.update_split_point(ctx.size(), mouse.pos);
                        ctx.request_layout();
                    }

                    if ctx.is_hot() && self.bar_hit_test(ctx.size(), mouse.pos) || ctx.is_active() {
                        match self.split_axis {
                            Axis::Horizontal => ctx.set_cursor(&Cursor::ResizeLeftRight),
                            Axis::Vertical => ctx.set_cursor(&Cursor::ResizeUpDown),
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

        match self.split_axis {
            Axis::Horizontal => {
                if !bc.is_width_bounded() {
                    log::warn!("A Split widget was given an unbounded width to split.")
                }
            }
            Axis::Vertical => {
                if !bc.is_height_bounded() {
                    log::warn!("A Split widget was given an unbounded height to split.")
                }
            }
        }

        let mut my_size = bc.max();
        let bar_area = self.bar_area();
        let reduced_size = Size::new(
            (my_size.width - bar_area).max(0.),
            (my_size.height - bar_area).max(0.),
        );

        // Update our effective split point to respect our constraints
        self.split_point_effective = {
            let (min_limit, max_limit) = self.split_side_limits(reduced_size);
            let reduced_axis_size = self.split_axis.major(reduced_size);
            if reduced_axis_size.is_infinite() || reduced_axis_size <= std::f64::EPSILON {
                0.5
            } else {
                clamp(
                    self.split_point_chosen,
                    min_limit / reduced_axis_size,
                    max_limit / reduced_axis_size,
                )
            }
        };

        let (child1_bc, child2_bc) = match self.split_axis {
            Axis::Horizontal => {
                let child1_width = (reduced_size.width * self.split_point_effective)
                    .floor()
                    .max(0.0);
                let child2_width = (reduced_size.width - child1_width).max(0.0);
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
            Axis::Vertical => {
                let child1_height = (reduced_size.height * self.split_point_effective)
                    .floor()
                    .max(0.0);
                let child2_height = (reduced_size.height - child1_height).max(0.0);
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

        // Top-left align for both children, out of laziness.
        // Reduce our unsplit direction to the larger of the two widgets
        let (child1_rect, child2_rect) = match self.split_axis {
            Axis::Horizontal => {
                my_size.height = child1_size.height.max(child2_size.height);
                (
                    Rect::from_origin_size(Point::ORIGIN, child1_size),
                    Rect::from_origin_size(
                        Point::new(child1_size.width + bar_area, 0.0),
                        child2_size,
                    ),
                )
            }
            Axis::Vertical => {
                my_size.width = child1_size.width.max(child2_size.width);
                (
                    Rect::from_origin_size(Point::ORIGIN, child1_size),
                    Rect::from_origin_size(
                        Point::new(0.0, child1_size.height + bar_area),
                        child2_size,
                    ),
                )
            }
        };
        self.child1.set_layout_rect(ctx, data, env, child1_rect);
        self.child2.set_layout_rect(ctx, data, env, child2_rect);

        let paint_rect = self.child1.paint_rect().union(self.child2.paint_rect());
        let insets = paint_rect - Rect::ZERO.with_size(my_size);
        ctx.set_paint_insets(insets);

        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.solid {
            self.paint_solid_bar(ctx, env);
        } else {
            self.paint_stroked_bar(ctx, env);
        }
        self.child1.paint(ctx, &data, env);
        self.child2.paint(ctx, &data, env);
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
