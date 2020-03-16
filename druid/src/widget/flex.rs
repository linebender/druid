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

use crate::widget::SizedBox;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, UpdateCtx, Widget, WidgetPod,
};

/// A container with either horizontal or vertical layout.
pub struct Flex<T> {
    direction: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    children: Vec<ChildWidget<T>>,
}

struct ChildWidget<T> {
    widget: WidgetPod<T, Box<dyn Widget<T>>>,
    params: Params,
}

/// A dummy widget we use to do spacing.
struct Spacer {
    axis: Axis,
    len: KeyOrValue<f64>,
}

#[derive(Clone, Copy)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

/// The alignment of the widgets on the container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossAxisAlignment {
    /// Top or leading.
    ///
    /// In a vertical container, widgets are top aligned. In a horiziontal
    /// container, their leading edges are aligned.
    Start,
    /// Widgets are centered in the container.
    Center,
    /// Bottom  or trailing.
    ///
    /// In a vertical container, widgets are bottom aligned. In a horiziontal
    /// container, their trailing edges are aligned.
    End,
}

/// Arrangement of children on the main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAxisAlignment {
    /// Top or leading.
    ///
    /// Children are aligned with the top or leading edge, without padding.
    Start,
    /// Children are centered, without padding.
    Center,
    /// Bottom or trailing.
    ///
    /// Children are aligned with the bottom or trailing edge, without padding.
    End,
    /// Extra space is divided evenly between each child.
    SpaceBetween,
    /// Extra space is divided evenly between each child, as well as at the ends.
    SpaceEvenly,
    /// Space between each child, with less at the start and end.
    ///
    /// This divides space such that each child is separated by `n` units,
    /// and the start and end have `n/2` units of padding.
    SpaceAround,
}

#[derive(Copy, Clone, Default)]
struct Params {
    flex: f64,
}

impl Axis {
    pub(crate) fn major(self, coords: Size) -> f64 {
        match self {
            Axis::Horizontal => coords.width,
            Axis::Vertical => coords.height,
        }
    }

    pub(crate) fn minor(self, coords: Size) -> f64 {
        match self {
            Axis::Horizontal => coords.height,
            Axis::Vertical => coords.width,
        }
    }

    pub(crate) fn pack(self, major: f64, minor: f64) -> (f64, f64) {
        match self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }

    /// Generate constraints with new values on the major axis.
    fn constraints(self, bc: &BoxConstraints, min_major: f64, major: f64) -> BoxConstraints {
        match self {
            Axis::Horizontal => BoxConstraints::new(
                Size::new(min_major, bc.min().height),
                Size::new(major, bc.max().height),
            ),
            Axis::Vertical => BoxConstraints::new(
                Size::new(bc.min().width, min_major),
                Size::new(bc.max().width, major),
            ),
        }
    }
}

impl<T: Data> Flex<T> {
    /// Create a new horizontal stack.
    ///
    /// The child widgets are laid out horizontally, from left to right.
    pub fn row() -> Self {
        Flex {
            direction: Axis::Horizontal,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Start,
            main_alignment: MainAxisAlignment::Start,
            fill_major_axis: false,
        }
    }

    /// Create a new vertical stack.
    ///
    /// The child widgets are laid out vertically, from top to bottom.
    pub fn column() -> Self {
        Flex {
            direction: Axis::Vertical,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Start,
            main_alignment: MainAxisAlignment::Start,
            fill_major_axis: false,
        }
    }

    /// Builder-style method for specifying the childrens' [`CrossAxisAlignment`].
    ///
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_alignment = alignment;
        self
    }

    /// Builder-style method for specifying the childrens' [`MainAxisAlignment`].
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_alignment = alignment;
        self
    }

    /// Builder-style method for setting whether the container must expand
    /// to fill the available space on its main axis.
    ///
    /// If any children have flex then this container will expand to fill all
    /// available space on its main axis; But if no children are flex,
    /// this flag determines whether or not the container should shrink to fit,
    /// or must expand to fill.
    ///
    /// If it expands, and there is extra space left over, that space is
    /// distributed in accordance with the [`MainAxisAlignment`].
    ///
    /// The default value is `false`.
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn must_fill_main_axis(mut self, fill: bool) -> Self {
        self.fill_major_axis = fill;
        self
    }

    /// Builder-style variant of `add_child`.
    ///
    /// Convenient for assembling a group of widgets in a single expression.
    pub fn with_child(mut self, child: impl Widget<T> + 'static, flex: f64) -> Self {
        self.add_child(child, flex);
        self
    }

    /// Builder-style method for adding a fixed-size spacer to the container.
    pub fn with_spacer(mut self, len: impl Into<KeyOrValue<f64>>) -> Self {
        self.add_spacer(len);
        self
    }

    /// Builder-style method for adding a `flex` spacer to the container.
    ///
    /// See [`add_child`] for an overview of `flex`.
    ///
    /// [`add_child`]: #method.add_child
    pub fn with_flex_spacer(mut self, flex: f64) -> Self {
        self.add_flex_spacer(flex);
        self
    }

    /// Set the childrens' [`CrossAxisAlignment`].
    ///
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn set_cross_axis_alignment(&mut self, alignment: CrossAxisAlignment) {
        self.cross_alignment = alignment;
    }

    /// Set the childrens' [`MainAxisAlignment`].
    ///
    /// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
    pub fn set_main_axis_alignment(&mut self, alignment: MainAxisAlignment) {
        self.main_alignment = alignment;
    }

    /// Set whether the container must expand to fill the available space on
    /// its main axis.
    pub fn set_must_fill_main_axis(&mut self, fill: bool) {
        self.fill_major_axis = fill;
    }

    /// Add a child widget.
    ///
    /// If `flex` is zero, then the child is non-flex. It is given the same
    /// constraints on the "minor axis" as its parent, but unconstrained on the
    /// "major axis".
    ///
    /// If `flex` is non-zero, then all the space left over after layout of
    /// the non-flex children is divided up, in proportion to the `flex` value,
    /// among the flex children.
    ///
    /// See also `with_child`.
    pub fn add_child(&mut self, child: impl Widget<T> + 'static, flex: f64) {
        let params = Params { flex };
        let child = ChildWidget {
            widget: WidgetPod::new(child).boxed(),
            params,
        };
        self.children.push(child);
    }

    /// Add an empty spacer widget with the given length.
    pub fn add_spacer(&mut self, len: impl Into<KeyOrValue<f64>>) {
        let spacer = Spacer {
            axis: self.direction,
            len: len.into(),
        };
        self.add_child(spacer, 0.0);
    }

    /// Add an empty spacer widget with a specific `flex` factor.
    ///
    /// See [`add_child`] for an overview of `flex`.
    ///
    /// [`add_child`]: #method.add_child
    pub fn add_flex_spacer(&mut self, flex: f64) {
        self.add_child(SizedBox::empty(), flex);
    }
}

impl<T: Data> Widget<T> for Flex<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        for child in &mut self.children {
            child.widget.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for child in &mut self.children {
            child.widget.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        for child in &mut self.children {
            child.widget.update(ctx, data, env);
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
        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        // Measure non-flex children.
        let mut total_non_flex = 0.0;
        let mut minor = self.direction.minor(bc.min());
        for child in &mut self.children {
            if child.params.flex == 0.0 {
                let child_bc = self
                    .direction
                    .constraints(&loosened_bc, 0.0, std::f64::INFINITY);
                let child_size = child.widget.layout(layout_ctx, &child_bc, data, env);
                minor = minor.max(self.direction.minor(child_size));
                total_non_flex += self.direction.major(child_size);
                // Stash size.
                child.widget.set_layout_rect(child_size.to_rect());
            }
        }

        let total_major = self.direction.major(bc.max());
        let remaining = (total_major - total_non_flex).max(0.0);
        let flex_sum: f64 = self.children.iter().map(|child| child.params.flex).sum();
        let mut flex_used: f64 = 0.0;

        // Measure flex children.
        for child in &mut self.children {
            if child.params.flex != 0.0 {
                let major = remaining * child.params.flex / flex_sum;

                let min_major = if major.is_infinite() { 0.0 } else { major };
                let child_bc = self.direction.constraints(&loosened_bc, min_major, major);
                let child_size = child.widget.layout(layout_ctx, &child_bc, data, env);
                flex_used += self.direction.major(child_size);
                minor = minor.max(self.direction.minor(child_size));
                // Stash size.
                child.widget.set_layout_rect(child_size.to_rect());
            }
        }

        // figure out if we have extra space on major axis, and if so how to use it
        let extra = if self.fill_major_axis {
            (remaining - flex_used).max(0.0)
        } else {
            // if we are *not* expected to fill our available space this usually
            // means we don't have any extra, unless dictated by our constraints.
            (self.direction.major(bc.min()) - (total_non_flex + flex_used)).max(0.0)
        };

        let spacing = self.main_alignment.spacing(extra, self.children.len());
        // Finalize layout, assigning positions to each child.
        let mut major = spacing.pre;
        let mut child_paint_rect = Rect::ZERO;
        for child in &mut self.children {
            let rect = child.widget.layout_rect();
            let extra_minor = minor - self.direction.minor(rect.size());
            let align_minor = self.cross_alignment.align(extra_minor);
            let pos: Point = self.direction.pack(major, align_minor).into();

            child.widget.set_layout_rect(rect.with_origin(pos));
            child_paint_rect = child_paint_rect.union(child.widget.paint_rect());
            major += self.direction.major(rect.size());
            major += spacing.between;
        }
        major -= spacing.between;
        major += spacing.post;

        if flex_sum > 0.0 && total_major.is_infinite() {
            log::warn!("A child of Flex is flex, but Flex is unbounded.")
        }

        if flex_sum > 0.0 {
            major = total_major;
        }

        let my_size: Size = self.direction.pack(major, minor).into();

        // if we don't have to fill the main axis, we loosen that axis before constraining
        let my_size = if !self.fill_major_axis {
            let max_major = self.direction.major(bc.max());
            self.direction
                .constraints(bc, 0.0, max_major)
                .constrain(my_size)
        } else {
            bc.constrain(my_size)
        };

        let my_bounds = my_size.to_rect();
        let insets = child_paint_rect - my_bounds;
        layout_ctx.set_paint_insets(insets);
        my_size
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        for child in &mut self.children {
            child.widget.paint_with_offset(paint_ctx, data, env);
        }
    }
}

impl CrossAxisAlignment {
    /// Given the difference between the size of the container and the size
    /// of the child (on their minor axis) return the necessary offset for
    /// this alignment.
    fn align(self, val: f64) -> f64 {
        match self {
            CrossAxisAlignment::Start => 0.0,
            CrossAxisAlignment::Center => val / 2.0,
            CrossAxisAlignment::End => val,
        }
    }
}

impl MainAxisAlignment {
    fn spacing(self, extra: f64, n_children: usize) -> Spacing {
        if extra.is_infinite() {
            return Spacing::default();
        }
        let (pre, between, post) = match self {
            MainAxisAlignment::Start => (0., 0., extra),
            MainAxisAlignment::End => (extra, 0., 0.),
            MainAxisAlignment::Center => (extra * 0.5, 0., extra * 0.5),
            MainAxisAlignment::SpaceBetween => {
                let space = match n_children {
                    0 | 1 => 0.0,
                    n => extra / (n - 1) as f64,
                };
                (0., space, 0.)
            }
            MainAxisAlignment::SpaceEvenly => {
                let n = (n_children + 1) as f64;
                let space = extra / n;
                (space, space, space)
            }
            MainAxisAlignment::SpaceAround => {
                let n = n_children as f64;
                let space = extra / n;
                (space / 2.0, space, space / 2.0)
            }
        };
        Spacing { pre, between, post }.assert_finite()
    }
}

#[derive(Debug, Default, Clone)]
struct Spacing {
    pre: f64,
    between: f64,
    post: f64,
}

impl Spacing {
    fn assert_finite(self) -> Self {
        assert!(self.pre.is_finite() && self.between.is_finite() && self.post.is_finite());
        self
    }
}

// we have these impls mostly for our 'flex' example, but I could imagine
// them being broadly useful?
impl Data for MainAxisAlignment {
    fn same(&self, other: &MainAxisAlignment) -> bool {
        self == other
    }
}

impl Data for CrossAxisAlignment {
    fn same(&self, other: &CrossAxisAlignment) -> bool {
        self == other
    }
}

impl<T: Data> Widget<T> for Spacer {
    fn event(&mut self, _: &mut EventCtx, _: &Event, _: &mut T, _: &Env) {}
    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &T, _: &Env) {}
    fn update(&mut self, _: &mut UpdateCtx, _: &T, _: &T, _: &Env) {}
    fn layout(&mut self, _: &mut LayoutCtx, _: &BoxConstraints, _: &T, env: &Env) -> Size {
        let major = self.len.resolve(env);
        self.axis.pack(major, 0.0).into()
    }
    fn paint(&mut self, _: &mut PaintCtx, _: &T, _: &Env) {}
}
