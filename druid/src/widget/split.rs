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

//! A widget which splits an area in two, with a set ratio.

use crate::kurbo::{Line, Point, Rect, Size};
use crate::widget::flex::Axis;
use crate::{
    theme, BoxConstraints, Cursor, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, RenderContext, UpdateCtx, Widget, WidgetPod,
};

///A container containing two other widgets, splitting the area either horizontally or vertically.
pub struct Split<T: Data> {
    split_direction: Axis,
    draggable: bool,
    split_point: f64,
    splitter_size: f64,
    child1: WidgetPod<T, Box<dyn Widget<T>>>,
    child2: WidgetPod<T, Box<dyn Widget<T>>>,
}

impl<T: Data> Split<T> {
    ///Create a new split panel.
    fn new(
        split_direction: Axis,
        child1: impl Widget<T> + 'static,
        child2: impl Widget<T> + 'static,
    ) -> Self {
        Split {
            split_direction,
            split_point: 0.5,
            splitter_size: 10.0,
            draggable: false,
            child1: WidgetPod::new(child1).boxed(),
            child2: WidgetPod::new(child2).boxed(),
        }
    }
    /// Create a new split panel, splitting the vertical dimension between two children.
    pub fn vertical(child1: impl Widget<T> + 'static, child2: impl Widget<T> + 'static) -> Self {
        Self::new(Axis::Vertical, child1, child2)
    }
    /// Create a new split panel, splitting the horizontal dimension between two children.
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
    fn splitter_hit_test(&self, size: Size, mouse_pos: Point) -> bool {
        match self.split_direction {
            Axis::Horizontal => {
                let center = size.width * self.split_point;
                (center - mouse_pos.x).abs() < self.splitter_size.min(5.0) / 2.0
            }
            Axis::Vertical => {
                let center = size.height * self.split_point;
                (center - mouse_pos.y).abs() < self.splitter_size.min(5.0) / 2.0
            }
        }
    }
    fn update_splitter(&mut self, size: Size, mouse_pos: Point) {
        self.split_point = match self.split_direction {
            Axis::Horizontal => {
                let max_limit = size.width - (self.splitter_size * 0.5).min(5.0);
                let min_limit = (self.splitter_size * 0.5).min(5.0);
                let max_split = max_limit / size.width;
                let min_split = min_limit / size.width;
                if mouse_pos.x > max_limit {
                    max_split
                } else if mouse_pos.x < min_limit {
                    min_split
                } else {
                    mouse_pos.x / size.width
                }
            }
            Axis::Vertical => {
                let max_limit = size.height - (self.splitter_size * 0.5).min(5.0);
                let min_limit = (self.splitter_size * 0.5).min(5.0);
                let max_split = max_limit / size.height;
                let min_split = min_limit / size.height;
                if mouse_pos.y > max_limit {
                    max_split
                } else if mouse_pos.y < min_limit {
                    min_split
                } else {
                    mouse_pos.y / size.height
                }
            }
        }
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
                        ctx.invalidate();
                    }
                }
                Event::MouseMoved(mouse) => {
                    if ctx.is_active() {
                        self.update_splitter(ctx.size(), mouse.pos);
                        ctx.invalidate();
                    }

                    if ctx.is_hot() && self.splitter_hit_test(ctx.size(), mouse.pos)
                        || ctx.is_active()
                    {
                        match self.split_direction {
                            Axis::Vertical => ctx.set_cursor(&Cursor::ResizeUpDown),
                            Axis::Horizontal => ctx.set_cursor(&Cursor::ResizeLeftRight),
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
            Axis::Horizontal => {
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
            Axis::Vertical => {
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
            Axis::Horizontal => {
                my_size.height = child1_size.height.max(child2_size.height);
                (
                    Rect::from_origin_size(Point::ORIGIN, child1_size),
                    Rect::from_origin_size(
                        Point::new(child1_size.width + self.splitter_size, 0.0),
                        child2_size,
                    ),
                )
            }
            Axis::Vertical => {
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
        my_size
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        let size = paint_ctx.size();
        //third, because we're putting the lines at roughly third points.
        //small, because we floor, to give the extra pixel (roughly) to the middle.
        let small_third = (self.splitter_size / 3.0).floor();
        let (line1, line2) = match self.split_direction {
            Axis::Horizontal => {
                let reduced_width = size.width - self.splitter_size;
                let edge1 = reduced_width * self.split_point;
                let edge2 = edge1 + self.splitter_size;
                (
                    Line::new(
                        Point::new(edge1 + small_third, 0.0),
                        Point::new(edge1 + small_third, size.height),
                    ),
                    Line::new(
                        Point::new(edge2 - small_third, 0.0),
                        Point::new(edge2 - small_third, size.height),
                    ),
                )
            }
            Axis::Vertical => {
                let reduced_height = size.height - self.splitter_size;
                let edge1 = reduced_height * self.split_point;
                let edge2 = edge1 + self.splitter_size;
                (
                    Line::new(
                        Point::new(0.0, edge1 + small_third),
                        Point::new(size.width, edge1 + small_third),
                    ),
                    Line::new(
                        Point::new(0.0, edge2 - small_third),
                        Point::new(size.width, edge2 - small_third),
                    ),
                )
            }
        };
        let line_color = if self.draggable {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER)
        };
        paint_ctx.stroke(line1, &line_color, 1.0);
        paint_ctx.stroke(line2, &line_color, 1.0);

        self.child1.paint_with_offset(paint_ctx, &data, env);
        self.child2.paint_with_offset(paint_ctx, &data, env);
    }
}
