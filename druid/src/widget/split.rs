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
    split_point_chosen: f64,
    split_point_effective: f64,
    splitter_size: f64,
    min_splitter_area: f64,
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
            split_point_chosen: 0.5,
            split_point_effective: 0.5,
            splitter_size: 6.0,
            min_splitter_area: 6.0,
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
        self.split_point_chosen = split_point;
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

    /// Set the width of the splitter bar.
    ///
    /// The value must be positive or zero.
    /// The default splitter size is `6.0`.
    pub fn splitter_size(mut self, splitter_size: f64) -> Self {
        assert!(
            splitter_size >= 0.0,
            "splitter_width must be 0.0 or greater!"
        );
        self.splitter_size = splitter_size;
        self
    }

    /// Set the minimum width of the splitter area.
    ///
    /// The minimum splitter area defines the minimum width of the area
    /// where mouse hit detection is done for the splitter.
    /// The final area is either this or the splitter size, whichever is greater.
    ///
    /// This can be useful when you want to use a very narrow visual splitter,
    /// but don't want to sacrifice user experience.
    ///
    /// The default minimum splitter area is `6.0`.
    pub fn min_splitter_area(mut self, min_splitter_area: f64) -> Self {
        assert!(
            min_splitter_area >= 0.0,
            "min_splitter_area must be 0.0 or greater!"
        );
        self.min_splitter_area = min_splitter_area;
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

    /// Returns the width of the area of mouse hit detection.
    #[inline]
    fn splitter_area(&self) -> f64 {
        self.splitter_size.max(self.min_splitter_area)
    }

    /// Returns the padding width added to the splitter on both sides.
    #[inline]
    fn splitter_padding(&self) -> f64 {
        (self.splitter_area() - self.splitter_size) / 2.0
    }

    /// Returns the location of the edges of the splitter area,
    /// given the specified total size.
    fn splitter_edges(&self, size: Size) -> (f64, f64) {
        let splitter_area = self.splitter_area();
        match self.split_direction {
            Axis::Vertical => {
                let reduced_width = size.width - splitter_area;
                let edge1 = (reduced_width * self.split_point_effective).floor();
                let edge2 = edge1 + splitter_area;
                (edge1, edge2)
            }
            Axis::Horizontal => {
                let reduced_height = size.height - splitter_area;
                let edge1 = (reduced_height * self.split_point_effective).floor();
                let edge2 = edge1 + splitter_area;
                (edge1, edge2)
            }
        }
    }

    /// Returns true if the provided mouse pos is inside the splitter area.
    fn splitter_hit_test(&self, size: Size, mouse_pos: Point) -> bool {
        let (edge1, edge2) = self.splitter_edges(size);
        match self.split_direction {
            Axis::Vertical => mouse_pos.x >= edge1 && mouse_pos.x <= edge2,
            Axis::Horizontal => mouse_pos.y >= edge1 && mouse_pos.y <= edge2,
        }
    }

    /// Returns the min and max split coordinate of the provided size.
    fn calculate_limits(&self, size: Size) -> (f64, f64) {
        // Since the Axis::Direction tells us the direction of the splitter itself
        // we need the minor axis to get the size of the split direction
        let size_in_split_direction = self.split_direction.minor(size);

        let mut min_limit = self.min_size;
        let mut max_limit = (size_in_split_direction - min_limit).max(0.0);

        if min_limit > max_limit {
            min_limit = 0.5 * (min_limit + max_limit);
            max_limit = min_limit;
        }

        (min_limit, max_limit)
    }

    /// Set a new chosen split point.
    fn update_split_point(&mut self, size: Size, mouse_pos: Point) {
        let (min_limit, max_limit) = self.calculate_limits(size);
        self.split_point_chosen = match self.split_direction {
            Axis::Vertical => clamp(mouse_pos.x, min_limit, max_limit) / size.width,
            Axis::Horizontal => clamp(mouse_pos.y, min_limit, max_limit) / size.height,
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
        let (edge1, edge2) = self.splitter_edges(size);
        let padding = self.splitter_padding();
        let rect = match self.split_direction {
            Axis::Vertical => Rect::from_points(
                Point::new(edge1 + padding.ceil(), 0.0),
                Point::new(edge2 - padding.floor(), size.height),
            ),
            Axis::Horizontal => Rect::from_points(
                Point::new(0.0, edge1 + padding.ceil()),
                Point::new(size.width, edge2 - padding.floor()),
            ),
        };
        let splitter_color = self.get_color(env);
        ctx.fill(rect, &splitter_color);
    }

    fn paint_stroked(&mut self, ctx: &mut PaintCtx, env: &Env) {
        let size = ctx.size();
        // Set the line width to a third of the splitter size,
        // because we'll paint two equal lines at the edges.
        let line_width = (self.splitter_size / 3.0).floor();
        let line_midpoint = line_width / 2.0;
        let (edge1, edge2) = self.splitter_edges(size);
        let padding = self.splitter_padding();
        let (line1, line2) = match self.split_direction {
            Axis::Vertical => (
                Line::new(
                    Point::new(edge1 + line_midpoint + padding.ceil(), 0.0),
                    Point::new(edge1 + line_midpoint + padding.ceil(), size.height),
                ),
                Line::new(
                    Point::new(edge2 - line_midpoint - padding.floor(), 0.0),
                    Point::new(edge2 - line_midpoint - padding.floor(), size.height),
                ),
            ),
            Axis::Horizontal => (
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
        let splitter_color = self.get_color(env);
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
                    if mouse.button.is_left() && self.splitter_hit_test(ctx.size(), mouse.pos) {
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
                Event::MouseMoved(mouse) => {
                    if ctx.is_active() {
                        self.update_split_point(ctx.size(), mouse.pos);
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

        // Update our effective split point to respect our constraints
        let (min_limit, max_limit) = self.calculate_limits(my_size);
        self.split_point_effective = match self.split_direction {
            Axis::Vertical => {
                if my_size.width <= std::f64::EPSILON {
                    0.5
                } else {
                    clamp(
                        self.split_point_chosen,
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
                        self.split_point_chosen,
                        min_limit / my_size.height,
                        max_limit / my_size.height,
                    )
                }
            }
        };

        let splitter_area = self.splitter_area();
        let reduced_width = my_size.width - splitter_area;
        let reduced_height = my_size.height - splitter_area;
        let (child1_bc, child2_bc) = match self.split_direction {
            Axis::Vertical => {
                if !bc.is_width_bounded() {
                    log::warn!("A Split widget was given an unbounded width to split.")
                }
                let child1_width = (reduced_width * self.split_point_effective)
                    .floor()
                    .max(0.0);
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
                let child1_height = (reduced_height * self.split_point_effective)
                    .floor()
                    .max(0.0);
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
                        Point::new(child1_size.width + splitter_area, 0.0),
                        child2_size,
                    ),
                )
            }
            Axis::Horizontal => {
                my_size.width = child1_size.width.max(child2_size.width);
                (
                    Rect::from_origin_size(Point::ORIGIN, child1_size),
                    Rect::from_origin_size(
                        Point::new(0.0, child1_size.height + splitter_area),
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
