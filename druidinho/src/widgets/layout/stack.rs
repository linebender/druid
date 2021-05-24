// Copyright 2021 The Druid Authors.
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

use crate::kurbo::{common::FloatExt, Point, Rect, Size};
use crate::piet::{Color, RenderContext};
use crate::widget_host::WidgetHost;
use crate::{BoxConstraints, EventCtx, LayoutCtx, MouseEvent, PaintCtx, Widget};
use druid_shell::{KeyEvent, TimerToken};

/// A container with either horizontal or vertical layout.
#[derive(Default)]
pub struct Stack<T: Axis> {
    children: Vec<WidgetHost<Box<dyn Widget>>>,
    axis: T,
}

/// A horizontal collection of widgets.
pub type Row = Stack<Horizontal>;

/// A vertical collection of widgets.
pub type Column = Stack<Vertical>;

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
pub trait Axis {
    type Cross: Axis;

    fn cross(&self) -> Self::Cross;
    fn major(&self, coords: Size) -> f64;
    fn minor(&self, coords: Size) -> f64 {
        self.cross().major(coords)
    }
    fn major_span(&self, rect: Rect) -> (f64, f64);
    fn minor_span(&self, rect: Rect) -> (f64, f64) {
        self.cross().minor_span(rect)
    }
    fn major_pos(&self, pos: Point) -> f64;
    fn minor_pos(&self, pos: Point) -> f64 {
        self.cross().major_pos(pos)
    }
    /// Arrange the major and minor measurements with respect to this axis
    /// such that it forms an (x, y) pair.
    fn pack(&self, major: f64, minor: f64) -> (f64, f64);

    /// Generate constraints with new values on the major axis.
    fn constraints(&self, bc: &BoxConstraints, min_major: f64, major: f64) -> BoxConstraints;
}

/// The horizontal axis.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Horizontal;

/// The vertical axis.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vertical;

impl Axis for Horizontal {
    type Cross = Vertical;

    fn cross(&self) -> Self::Cross {
        Vertical
    }

    fn major(&self, coords: Size) -> f64 {
        coords.width
    }

    fn major_span(&self, rect: Rect) -> (f64, f64) {
        (rect.x0, rect.x1)
    }

    fn major_pos(&self, pos: Point) -> f64 {
        pos.x
    }

    fn pack(&self, major: f64, minor: f64) -> (f64, f64) {
        (major, minor)
    }

    fn constraints(&self, bc: &BoxConstraints, min_major: f64, major: f64) -> BoxConstraints {
        BoxConstraints::new(
            Size::new(min_major, bc.min().height),
            Size::new(major, bc.max().height),
        )
    }
}

impl Axis for Vertical {
    type Cross = Horizontal;

    fn cross(&self) -> Self::Cross {
        Horizontal
    }

    fn major(&self, coords: Size) -> f64 {
        coords.height
    }

    fn major_span(&self, rect: Rect) -> (f64, f64) {
        (rect.y0, rect.y1)
    }

    fn major_pos(&self, pos: Point) -> f64 {
        pos.y
    }

    fn pack(&self, major: f64, minor: f64) -> (f64, f64) {
        (minor, major)
    }

    fn constraints(&self, bc: &BoxConstraints, min_major: f64, major: f64) -> BoxConstraints {
        BoxConstraints::new(
            Size::new(bc.min().width, min_major),
            Size::new(bc.max().width, major),
        )
    }
}

impl<T: Axis + Default> Stack<T> {
    /// Create a new collection.
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T: Axis> Stack<T> {
    /// Builder-style variant of `add_child`.
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(mut self, child: impl Widget + 'static) -> Self {
        self.add_child(child);
        self
    }

    /// Add a child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: Flex::with_child
    pub fn add_child(&mut self, child: impl Widget + 'static) {
        let child: Box<dyn Widget> = Box::new(child);
        let child = WidgetHost::new(child);
        self.children.push(child);
    }
}

impl<T: Axis> Widget for Stack<T> {
    fn init(&mut self, ctx: &mut EventCtx) {
        self.children.iter_mut().for_each(|chld| chld.init(ctx))
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.mouse_down(ctx, event))
    }
    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.mouse_up(ctx, event))
    }
    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.mouse_move(ctx, event))
    }
    fn scroll(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.scroll(ctx, event))
    }
    fn key_down(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.key_down(ctx, event))
    }
    fn key_up(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.key_up(ctx, event))
    }
    fn timer(&mut self, ctx: &mut EventCtx, token: TimerToken) {
        self.children
            .iter_mut()
            .for_each(|chld| chld.timer(ctx, token))
    }
    fn paint(&self, ctx: &mut PaintCtx) {
        self.children.iter().for_each(|chld| chld.paint(ctx))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        bc.debug_check("Stack");
        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        let mut major_sum = 0.0;
        let mut minor_max = 0.0f64;
        for child in &mut self.children {
            let child_bc = self.axis.constraints(&loosened_bc, 0.0, std::f64::INFINITY);
            let child_size = child.layout(ctx, child_bc);
            let child_origin = self.axis.pack(major_sum, 0.0);
            child.set_origin(child_origin.into());
            major_sum += self.axis.major(child_size).expand();
            minor_max = minor_max.max(self.axis.minor(child_size).expand());
        }

        let measured_size: Size = self.axis.pack(major_sum, minor_max).into();
        let my_size = bc.constrain(measured_size);
        if measured_size.width > my_size.width || measured_size.height > my_size.height {
            eprintln!("stack children don't fit :(")
        }

        my_size
    }
}
