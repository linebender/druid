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

//! A widget that arranges its children in a one-dimensional array.

use crate::kurbo::{Point, Rect, Size};

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
    WidgetPod,
};

/// A builder for a row widget that can contain flex children.
///
/// The actual widget is implemented by [`Flex`], but this builder
/// is provided for convenience.
///
/// [`Flex`]: struct.Flex.html
pub struct Row;
/// A builder for a column widget that can contain flex children.
///
/// The actual widget is implemented by [`Flex`], but this builder
/// is provided for convenience.
///
/// [`Flex`]: struct.Flex.html
pub struct Column;

/// A container with either horizontal or vertical layout.
pub struct Flex<T: Data> {
    direction: Axis,

    children: Vec<ChildWidget<T>>,
}

struct ChildWidget<T: Data> {
    widget: WidgetPod<T, Box<dyn Widget<T>>>,
    params: Params,
}

pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Default)]
struct Params {
    flex: f64,
}

impl Axis {
    fn major(&self, coords: Size) -> f64 {
        match *self {
            Axis::Horizontal => coords.width,
            Axis::Vertical => coords.height,
        }
    }

    fn minor(&self, coords: Size) -> f64 {
        match *self {
            Axis::Horizontal => coords.height,
            Axis::Vertical => coords.width,
        }
    }

    fn pack(&self, major: f64, minor: f64) -> (f64, f64) {
        match *self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }
}

impl Row {
    /// Create a new row widget.
    ///
    /// The child widgets are laid out horizontally, from left to right.
    pub fn new<T: Data>() -> Flex<T> {
        Flex {
            direction: Axis::Horizontal,

            children: Vec::new(),
        }
    }
}

impl Column {
    /// Create a new row widget.
    ///
    /// The child widgets are laid out vertically, from top to bottom.
    pub fn new<T: Data>() -> Flex<T> {
        Flex {
            direction: Axis::Vertical,

            children: Vec::new(),
        }
    }
}

impl<T: Data> Flex<T> {
    /// Add a child widget.
    ///
    /// If `flex` is zero, then the child is non-flex. It is given the same
    /// constraints on the "minor axis" as its parent, but unconstrained on the
    /// "major axis".
    ///
    /// If `flex` is non-zero, then all the space left over after layout of
    /// the non-flex children is divided up, in proportion to the `flex` value,
    /// among the flex children.
    pub fn add_child(&mut self, child: impl Widget<T> + 'static, flex: f64) {
        let params = Params { flex };
        let child = ChildWidget {
            widget: WidgetPod::new(child).boxed(),
            params,
        };
        self.children.push(child);
    }
}

impl<T: Data> Widget<T> for Flex<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, _base_state: &BaseState, data: &T, env: &Env) {
        for child in &mut self.children {
            child.widget.paint_with_offset(paint_ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Flex");

        // Measure non-flex children.
        let mut total_non_flex = 0.0;
        let mut minor = self.direction.minor(bc.min());
        for child in &mut self.children {
            if child.params.flex == 0.0 {
                let child_bc = match self.direction {
                    Axis::Horizontal => BoxConstraints::new(
                        Size::new(0.0, bc.min().height),
                        Size::new(std::f64::INFINITY, bc.max.height),
                    ),
                    Axis::Vertical => BoxConstraints::new(
                        Size::new(bc.min().width, 0.0),
                        Size::new(bc.max().width, std::f64::INFINITY),
                    ),
                };
                let child_size = child.widget.layout(layout_ctx, &child_bc, data, env);
                minor = minor.max(self.direction.minor(child_size));
                total_non_flex += self.direction.major(child_size);
                // Stash size.
                let rect = Rect::from_origin_size(Point::ORIGIN, child_size);
                child.widget.set_layout_rect(rect);
            }
        }

        let total_major = self.direction.major(bc.max());
        let remaining = (total_major - total_non_flex).max(0.0);
        let flex_sum: f64 = self.children.iter().map(|child| child.params.flex).sum();

        // Measure flex children.
        for child in &mut self.children {
            if child.params.flex != 0.0 {
                let major = remaining * child.params.flex / flex_sum;

                let min_major = if major.is_infinite() { 0.0 } else { major };

                let child_bc = match self.direction {
                    Axis::Horizontal => BoxConstraints::new(
                        Size::new(min_major, bc.min().height),
                        Size::new(major, bc.max().height),
                    ),
                    Axis::Vertical => BoxConstraints::new(
                        Size::new(bc.min().width, min_major),
                        Size::new(bc.max().width, major),
                    ),
                };
                let child_size = child.widget.layout(layout_ctx, &child_bc, data, env);
                minor = minor.max(self.direction.minor(child_size));
                // Stash size.
                let rect = Rect::from_origin_size(Point::ORIGIN, child_size);
                child.widget.set_layout_rect(rect);
            }
        }

        // Finalize layout, assigning positions to each child.
        let mut major = 0.0;
        for child in &mut self.children {
            // top-align, could do center etc. based on child height
            let rect = child.widget.get_layout_rect();
            let pos: Point = self.direction.pack(major, 0.0).into();
            child.widget.set_layout_rect(rect.with_origin(pos));
            major += self.direction.major(rect.size());
        }

        if flex_sum > 0.0 && total_major.is_infinite() {
            log::warn!("A child of Flex is flex, but Flex is unbounded.")
        }

        if flex_sum > 0.0 {
            major = total_major;
        }

        // TODO: should be able to make this `into`
        let (width, height) = self.direction.pack(major, minor);
        Size::new(width, height)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        for child in &mut self.children {
            child.widget.event(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&T>, data: &T, env: &Env) {
        for child in &mut self.children {
            child.widget.update(ctx, data, env);
        }
    }
}
