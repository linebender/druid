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

//! A widget that arranges its children in a one-dimensional array.

use crate::kurbo::common::FloatExt;
use crate::widget::prelude::*;
use crate::{Data, KeyOrValue, Point, Rect, WidgetPod};
use tracing::{instrument, trace};

/// A container with either horizontal or vertical layout.
///
/// This widget is the foundation of most layouts, and is highly configurable.
///
/// # Flex layout algorithm
///
/// Children of a `Flex` container can have an optional `flex` parameter.
/// Layout occurs in several passes. First we measure (calling their [`layout`]
/// method) our non-flex children, providing them with unbounded space on the
/// main axis. Next, the remaining space is divided between the flex children
/// according to their flex factor, and they are measured. Unlike a non-flex
/// child, a child with a non-zero flex factor has a maximum allowed size
/// on the main axis; non-flex children are allowed to choose their size first,
/// and freely.
///
/// If you would like a child to be forced to use up all of the flex space
/// passed to it, you can place it in a [`SizedBox`] set to `expand` in the
/// appropriate axis. There are convenience methods for this available on
/// [`WidgetExt`]: [`expand_width`] and [`expand_height`].
///
/// # Flex or non-flex?
///
/// When should your children be flexible? With other things being equal,
/// a flexible child has lower layout priority than a non-flexible child.
/// Imagine, for instance, we have a row that is 30dp wide, and we have
/// two children, both of which want to be 20dp wide. If child #1 is non-flex
/// and child #2 is flex, the first widget will take up its 20dp, and the second
/// widget will be constrained to 10dp.
///
/// If, instead, both widgets are flex, they will each be given equal space,
/// and both will end up taking up 15dp.
///
/// If both are non-flex they will both take up 20dp, and will overflow the
/// container.
///
/// ```no_compile
///  -------non-flex----- -flex-----
/// |       child #1     | child #2 |
///
///
///  ----flex------- ----flex-------
/// |    child #1   |    child #2   |
///
/// ```
///
/// In general, if you are using widgets that are opinionated about their size
/// (such as most control widgets, which are designed to lay out nicely together,
/// or text widgets that are sized to fit their text) you should make them
/// non-flexible.
///
/// If you are trying to divide space evenly, or if you want a particular item
/// to have access to all left over space, then you should make it flexible.
///
/// **note**: by default, a widget will not necessarily use all the space that
/// is available to it. For instance, the [`TextBox`] widget has a default
/// width, and will choose this width if possible, even if more space is
/// available to it. If you want to force a widget to use all available space,
/// you should expand it, with [`expand_width`] or [`expand_height`].
///
///
/// # Options
///
/// To experiment with these options, see the `flex` example in `druid/examples`.
///
/// - [`CrossAxisAlignment`] determines how children are positioned on the
/// cross or 'minor' axis. The default is `CrossAxisAlignment::Center`.
///
/// - [`MainAxisAlignment`] determines how children are positioned on the main
/// axis; this is only meaningful if the container has more space on the main
/// axis than is taken up by its children.
///
/// - [`must_fill_main_axis`] determines whether the container is obliged to
/// be maximally large on the major axis, as determined by its own constraints.
/// If this is `true`, then the container must fill the available space on that
/// axis; otherwise it may be smaller if its children are smaller.
///
/// Additional options can be set (or overridden) in the [`FlexParams`].
///
/// # Examples
///
/// Construction with builder methods
///
/// ```
/// use druid::widget::{Flex, FlexParams, Label, Slider, CrossAxisAlignment};
///
/// let my_row = Flex::row()
///     .cross_axis_alignment(CrossAxisAlignment::Center)
///     .must_fill_main_axis(true)
///     .with_child(Label::new("hello"))
///     .with_default_spacer()
///     .with_flex_child(Slider::new(), 1.0);
/// ```
///
/// Construction with mutating methods
///
/// ```
/// use druid::widget::{Flex, FlexParams, Label, Slider, CrossAxisAlignment};
///
/// let mut my_row = Flex::row();
/// my_row.set_must_fill_main_axis(true);
/// my_row.set_cross_axis_alignment(CrossAxisAlignment::Center);
/// my_row.add_child(Label::new("hello"));
/// my_row.add_default_spacer();
/// my_row.add_flex_child(Slider::new(), 1.0);
/// ```
///
/// [`layout`]: ../trait.Widget.html#tymethod.layout
/// [`MainAxisAlignment`]: enum.MainAxisAlignment.html
/// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
/// [`must_fill_main_axis`]: struct.Flex.html#method.must_fill_main_axis
/// [`FlexParams`]: struct.FlexParams.html
/// [`WidgetExt`]: ../trait.WidgetExt.html
/// [`expand_height`]: ../trait.WidgetExt.html#method.expand_height
/// [`expand_width`]: ../trait.WidgetExt.html#method.expand_width
/// [`TextBox`]: struct.TextBox.html
/// [`SizedBox`]: struct.SizedBox.html
pub struct Flex<T> {
    direction: Axis,
    cross_alignment: CrossAxisAlignment,
    main_alignment: MainAxisAlignment,
    fill_major_axis: bool,
    children: Vec<Child<T>>,
}

/// Optional parameters for an item in a [`Flex`] container (row or column).
///
/// Generally, when you would like to add a flexible child to a container,
/// you can simply call [`with_flex_child`] or [`add_flex_child`], passing the
/// child and the desired flex factor as a `f64`, which has an impl of
/// `Into<FlexParams>`.
///
/// If you need to set additional paramaters, such as a custom [`CrossAxisAlignment`],
/// you can construct `FlexParams` directly. By default, the child has the
/// same `CrossAxisAlignment` as the container.
///
/// For an overview of the flex layout algorithm, see the [`Flex`] docs.
///
/// # Examples
/// ```
/// use druid::widget::{FlexParams, Label, CrossAxisAlignment};
///
/// let mut row = druid::widget::Flex::<()>::row();
/// let child_1 = Label::new("I'm hungry");
/// let child_2 = Label::new("I'm scared");
/// // normally you just use a float:
/// row.add_flex_child(child_1, 1.0);
/// // you can construct FlexParams if needed:
/// let params = FlexParams::new(2.0, CrossAxisAlignment::End);
/// row.add_flex_child(child_2, params);
/// ```
///
/// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
/// [`Flex`]: struct.Flex.html
/// [`with_flex_child`]: struct.Flex.html#method.with_flex_child
/// [`add_flex_child`]: struct.Flex.html#method.add_flex_child
#[derive(Copy, Clone, Default)]
pub struct FlexParams {
    flex: f64,
    alignment: Option<CrossAxisAlignment>,
}

/// An axis in visual space.
///
/// Most often used by widgets to describe
/// the direction in which they grow as their number of children increases.
/// Has some methods for manipulating geometry with respect to the axis.
#[derive(Data, Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    /// The x axis
    Horizontal,
    /// The y axis
    Vertical,
}

impl Axis {
    /// Get the axis perpendicular to this one.
    pub fn cross(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    /// Extract from the argument the magnitude along this axis
    pub fn major(self, coords: Size) -> f64 {
        match self {
            Axis::Horizontal => coords.width,
            Axis::Vertical => coords.height,
        }
    }

    /// Extract from the argument the magnitude along the perpendicular axis
    pub fn minor(self, coords: Size) -> f64 {
        self.cross().major(coords)
    }

    /// Extract the extent of the argument in this axis as a pair.
    pub fn major_span(self, rect: Rect) -> (f64, f64) {
        match self {
            Axis::Horizontal => (rect.x0, rect.x1),
            Axis::Vertical => (rect.y0, rect.y1),
        }
    }

    /// Extract the extent of the argument in the minor axis as a pair.
    pub fn minor_span(self, rect: Rect) -> (f64, f64) {
        self.cross().major_span(rect)
    }

    /// Extract the coordinate locating the argument with respect to this axis.
    pub fn major_pos(self, pos: Point) -> f64 {
        match self {
            Axis::Horizontal => pos.x,
            Axis::Vertical => pos.y,
        }
    }

    /// Extract the coordinate locating the argument with respect to the perpendicular axis.
    pub fn minor_pos(self, pos: Point) -> f64 {
        self.cross().major_pos(pos)
    }

    /// Arrange the major and minor measurements with respect to this axis such that it forms
    /// an (x, y) pair.
    pub fn pack(self, major: f64, minor: f64) -> (f64, f64) {
        match self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }

    /// Generate constraints with new values on the major axis.
    pub(crate) fn constraints(
        self,
        bc: &BoxConstraints,
        min_major: f64,
        major: f64,
    ) -> BoxConstraints {
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

/// The alignment of the widgets on the container's cross (or minor) axis.
///
/// If a widget is smaller than the container on the minor axis, this determines
/// where it is positioned.
#[derive(Debug, Clone, Copy, PartialEq, Data)]
pub enum CrossAxisAlignment {
    /// Top or leading.
    ///
    /// In a vertical container, widgets are top aligned. In a horiziontal
    /// container, their leading edges are aligned.
    Start,
    /// Widgets are centered in the container.
    Center,
    /// Bottom or trailing.
    ///
    /// In a vertical container, widgets are bottom aligned. In a horiziontal
    /// container, their trailing edges are aligned.
    End,
    /// Align on the baseline.
    ///
    /// In a horizontal container, widgets are aligned along the calculated
    /// baseline. In a vertical container, this is equivalent to `End`.
    ///
    /// The calculated baseline is the maximum baseline offset of the children.
    Baseline,
    /// Fill the available space.
    ///
    /// The size on this axis is the size of the largest widget;
    /// other widgets must fill that space.
    Fill,
}

/// Arrangement of children on the main axis.
///
/// If there is surplus space on the main axis after laying out children, this
/// enum represents how children are laid out in this space.
#[derive(Debug, Clone, Copy, PartialEq, Data)]
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

impl FlexParams {
    /// Create custom `FlexParams` with a specific `flex_factor` and an optional
    /// [`CrossAxisAlignment`].
    ///
    /// You likely only need to create these manually if you need to specify
    /// a custom alignment; if you only need to use a custom `flex_factor` you
    /// can pass an `f64` to any of the functions that take `FlexParams`.
    ///
    /// By default, the widget uses the alignment of its parent [`Flex`] container.
    ///
    ///
    /// [`Flex`]: struct.Flex.html
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn new(flex: f64, alignment: impl Into<Option<CrossAxisAlignment>>) -> Self {
        FlexParams {
            flex,
            alignment: alignment.into(),
        }
    }
}

impl<T: Data> Flex<T> {
    /// Create a new Flex oriented along the provided axis.
    pub fn for_axis(axis: Axis) -> Self {
        Flex {
            direction: axis,
            children: Vec::new(),
            cross_alignment: CrossAxisAlignment::Center,
            main_alignment: MainAxisAlignment::Start,
            fill_major_axis: false,
        }
    }

    /// Create a new horizontal stack.
    ///
    /// The child widgets are laid out horizontally, from left to right.
    ///
    pub fn row() -> Self {
        Self::for_axis(Axis::Horizontal)
    }

    /// Create a new vertical stack.
    ///
    /// The child widgets are laid out vertically, from top to bottom.
    pub fn column() -> Self {
        Self::for_axis(Axis::Vertical)
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
    pub fn with_child(mut self, child: impl Widget<T> + 'static) -> Self {
        self.add_flex_child(child, 0.0);
        self
    }

    /// Builder-style method to add a flexible child to the container.
    ///
    /// This method is used when you need more control over the behaviour
    /// of the widget you are adding. In the general case, this likely
    /// means giving that child a 'flex factor', but it could also mean
    /// giving the child a custom [`CrossAxisAlignment`], or a combination
    /// of the two.
    ///
    /// This function takes a child widget and [`FlexParams`]; importantly
    /// you can pass in a float as your [`FlexParams`] in most cases.
    ///
    /// For the non-builder varient, see [`add_flex_child`].
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::{Flex, FlexParams, Label, Slider, CrossAxisAlignment};
    ///
    /// let my_row = Flex::row()
    ///     .with_flex_child(Slider::new(), 1.0)
    ///     .with_flex_child(Slider::new(), FlexParams::new(1.0, CrossAxisAlignment::End));
    /// ```
    ///
    /// [`FlexParams`]: struct.FlexParams.html
    /// [`add_flex_child`]: #method.add_flex_child
    /// [`CrossAxisAlignment`]: enum.CrossAxisAlignment.html
    pub fn with_flex_child(
        mut self,
        child: impl Widget<T> + 'static,
        params: impl Into<FlexParams>,
    ) -> Self {
        self.add_flex_child(child, params);
        self
    }

    /// Builder-style method to add a spacer widget with a standard size.
    ///
    /// The actual value of this spacer depends on whether this container is
    /// a row or column, as well as theme settings.
    pub fn with_default_spacer(mut self) -> Self {
        self.add_default_spacer();
        self
    }

    /// Builder-style method for adding a fixed-size spacer to the container.
    ///
    /// If you are laying out standard controls in this container, you should
    /// generally prefer to use [`add_default_spacer`].
    ///
    /// [`add_default_spacer`]: #method.add_default_spacer
    pub fn with_spacer(mut self, len: impl Into<KeyOrValue<f64>>) -> Self {
        self.add_spacer(len);
        self
    }

    /// Builder-style method for adding a `flex` spacer to the container.
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

    /// Add a non-flex child widget.
    ///
    /// See also [`with_child`].
    ///
    /// [`with_child`]: #method.with_child
    pub fn add_child(&mut self, child: impl Widget<T> + 'static) {
        self.add_flex_child(child, 0.0);
    }

    /// Add a flexible child widget.
    ///
    /// This method is used when you need more control over the behaviour
    /// of the widget you are adding. In the general case, this likely
    /// means giving that child a 'flex factor', but it could also mean
    /// giving the child a custom [`CrossAxisAlignment`], or a combination
    /// of the two.
    ///
    /// This function takes a child widget and [`FlexParams`]; importantly
    /// you can pass in a float as your [`FlexParams`] in most cases.
    ///
    /// For the builder-style varient, see [`with_flex_child`].
    ///
    /// # Examples
    ///
    /// ```
    /// use druid::widget::{Flex, FlexParams, Label, Slider, CrossAxisAlignment};
    ///
    /// let mut my_row = Flex::row();
    /// my_row.add_flex_child(Slider::new(), 1.0);
    /// my_row.add_flex_child(Slider::new(), FlexParams::new(1.0, CrossAxisAlignment::End));
    /// ```
    ///
    /// [`FlexParams`]: struct.FlexParams.html
    /// [`with_flex_child`]: #method.with_flex_child
    pub fn add_flex_child(
        &mut self,
        child: impl Widget<T> + 'static,
        params: impl Into<FlexParams>,
    ) {
        let params = params.into();
        let child = if params.flex == 0.0 {
            Child::Fixed {
                widget: WidgetPod::new(Box::new(child)),
                alignment: params.alignment,
            }
        } else {
            Child::Flex {
                widget: WidgetPod::new(Box::new(child)),
                alignment: params.alignment,
                flex: params.flex,
            }
        };
        self.children.push(child);
    }

    /// Add a spacer widget with a standard size.
    ///
    /// The actual value of this spacer depends on whether this container is
    /// a row or column, as well as theme settings.
    pub fn add_default_spacer(&mut self) {
        let key = match self.direction {
            Axis::Vertical => crate::theme::WIDGET_PADDING_VERTICAL,
            Axis::Horizontal => crate::theme::WIDGET_PADDING_HORIZONTAL,
        };
        self.add_spacer(key);
    }

    /// Add an empty spacer widget with the given size.
    ///
    /// If you are laying out standard controls in this container, you should
    /// generally prefer to use [`add_default_spacer`].
    ///
    /// [`add_default_spacer`]: #method.add_default_spacer
    pub fn add_spacer(&mut self, len: impl Into<KeyOrValue<f64>>) {
        let value = len.into();
        let new_child = Child::FixedSpacer(value, 0.0);
        self.children.push(new_child);
    }

    /// Add an empty spacer widget with a specific `flex` factor.
    pub fn add_flex_spacer(&mut self, flex: f64) {
        let new_child = Child::FlexedSpacer(flex, 0.0);
        self.children.push(new_child);
    }
}

impl<T: Data> Widget<T> for Flex<T> {
    #[instrument(name = "Flex", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.event(ctx, event, data, env);
        }
    }

    #[instrument(name = "Flex", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.lifecycle(ctx, event, data, env);
        }
    }

    #[instrument(name = "Flex", level = "trace", skip(self, ctx, _old_data, data, env))]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.update(ctx, data, env);
        }
    }

    #[instrument(name = "Flex", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("Flex");
        // we loosen our constraints when passing to children.
        let loosened_bc = bc.loosen();

        // minor-axis values for all children
        let mut minor = self.direction.minor(bc.min());
        // these two are calculated but only used if we're baseline aligned
        let mut max_above_baseline = 0f64;
        let mut max_below_baseline = 0f64;
        let mut any_use_baseline = self.cross_alignment == CrossAxisAlignment::Baseline;

        // Measure non-flex children.
        let mut major_non_flex = 0.0;
        let mut flex_sum = 0.0;
        for child in &mut self.children {
            match child {
                Child::Fixed { widget, alignment } => {
                    any_use_baseline &= *alignment == Some(CrossAxisAlignment::Baseline);

                    let child_bc =
                        self.direction
                            .constraints(&loosened_bc, 0.0, std::f64::INFINITY);
                    let child_size = widget.layout(ctx, &child_bc, data, env);
                    let baseline_offset = widget.baseline_offset();

                    if child_size.width.is_infinite() {
                        tracing::warn!("A non-Flex child has an infinite width.");
                    }

                    if child_size.height.is_infinite() {
                        tracing::warn!("A non-Flex child has an infinite height.");
                    }

                    major_non_flex += self.direction.major(child_size).expand();
                    minor = minor.max(self.direction.minor(child_size).expand());
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Child::FixedSpacer(kv, calculated_siz) => {
                    *calculated_siz = kv.resolve(env);
                    major_non_flex += *calculated_siz;
                }
                Child::Flex { flex, .. } | Child::FlexedSpacer(flex, _) => flex_sum += *flex,
            }
        }

        let total_major = self.direction.major(bc.max());
        let remaining = (total_major - major_non_flex).max(0.0);
        let mut remainder: f64 = 0.0;

        let mut major_flex: f64 = 0.0;
        let px_per_flex = remaining / flex_sum;
        // Measure flex children.
        for child in &mut self.children {
            match child {
                Child::Flex { widget, flex, .. } => {
                    let desired_major = (*flex) * px_per_flex + remainder;
                    let actual_major = desired_major.round();
                    remainder = desired_major - actual_major;

                    let child_bc = self.direction.constraints(&loosened_bc, 0.0, actual_major);
                    let child_size = widget.layout(ctx, &child_bc, data, env);
                    let baseline_offset = widget.baseline_offset();

                    major_flex += self.direction.major(child_size).expand();
                    minor = minor.max(self.direction.minor(child_size).expand());
                    max_above_baseline =
                        max_above_baseline.max(child_size.height - baseline_offset);
                    max_below_baseline = max_below_baseline.max(baseline_offset);
                }
                Child::FlexedSpacer(flex, calculated_size) => {
                    let desired_major = (*flex) * px_per_flex + remainder;
                    *calculated_size = desired_major.round();
                    remainder = desired_major - *calculated_size;
                    major_flex += *calculated_size;
                }
                _ => {}
            }
        }

        // figure out if we have extra space on major axis, and if so how to use it
        let extra = if self.fill_major_axis {
            (remaining - major_flex).max(0.0)
        } else {
            // if we are *not* expected to fill our available space this usually
            // means we don't have any extra, unless dictated by our constraints.
            (self.direction.major(bc.min()) - (major_non_flex + major_flex)).max(0.0)
        };

        let mut spacing = Spacing::new(self.main_alignment, extra, self.children.len());

        // the actual size needed to tightly fit the children on the minor axis.
        // Unlike the 'minor' var, this ignores the incoming constraints.
        let minor_dim = match self.direction {
            Axis::Horizontal if any_use_baseline => max_below_baseline + max_above_baseline,
            _ => minor,
        };

        let extra_height = minor - minor_dim.min(minor);

        let mut major = spacing.next().unwrap_or(0.);
        let mut child_paint_rect = Rect::ZERO;

        for child in &mut self.children {
            match child {
                Child::Fixed { widget, alignment }
                | Child::Flex {
                    widget, alignment, ..
                } => {
                    let child_size = widget.layout_rect().size();
                    let alignment = alignment.unwrap_or(self.cross_alignment);
                    let child_minor_offset = match alignment {
                        // This will ignore baseline alignment if it is overridden on children,
                        // but is not the default for the container. Is this okay?
                        CrossAxisAlignment::Baseline
                            if matches!(self.direction, Axis::Horizontal) =>
                        {
                            let child_baseline = widget.baseline_offset();
                            let child_above_baseline = child_size.height - child_baseline;
                            extra_height + (max_above_baseline - child_above_baseline)
                        }
                        CrossAxisAlignment::Fill => {
                            let fill_size: Size = self
                                .direction
                                .pack(self.direction.major(child_size), minor_dim)
                                .into();
                            let child_bc = BoxConstraints::tight(fill_size);
                            widget.layout(ctx, &child_bc, data, env);
                            0.0
                        }
                        _ => {
                            let extra_minor = minor_dim - self.direction.minor(child_size);
                            alignment.align(extra_minor)
                        }
                    };

                    let child_pos: Point = self.direction.pack(major, child_minor_offset).into();
                    widget.set_origin(ctx, data, env, child_pos);
                    child_paint_rect = child_paint_rect.union(widget.paint_rect());
                    major += self.direction.major(child_size).expand();
                    major += spacing.next().unwrap_or(0.);
                }
                Child::FlexedSpacer(_, calculated_size)
                | Child::FixedSpacer(_, calculated_size) => {
                    major += *calculated_size;
                }
            }
        }

        if flex_sum > 0.0 && total_major.is_infinite() {
            tracing::warn!("A child of Flex is flex, but Flex is unbounded.")
        }

        if flex_sum > 0.0 {
            major = total_major;
        }

        let my_size: Size = self.direction.pack(major, minor_dim).into();

        // if we don't have to fill the main axis, we loosen that axis before constraining
        let my_size = if !self.fill_major_axis {
            let max_major = self.direction.major(bc.max());
            self.direction
                .constraints(bc, 0.0, max_major)
                .constrain(my_size)
        } else {
            bc.constrain(my_size)
        };

        let my_bounds = Rect::ZERO.with_size(my_size);
        let insets = child_paint_rect - my_bounds;
        ctx.set_paint_insets(insets);

        let baseline_offset = match self.direction {
            Axis::Horizontal => max_below_baseline,
            Axis::Vertical => (&self.children)
                .last()
                .map(|last| {
                    let child = last.widget();
                    if let Some(widget) = child {
                        let child_bl = widget.baseline_offset();
                        let child_max_y = widget.layout_rect().max_y();
                        let extra_bottom_padding = my_size.height - child_max_y;
                        child_bl + extra_bottom_padding
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0),
        };

        ctx.set_baseline_offset(baseline_offset);
        trace!(
            "Computed layout: size={}, baseline_offset={}",
            my_size,
            baseline_offset
        );
        my_size
    }

    #[instrument(name = "Flex", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        for child in self.children.iter_mut().filter_map(|x| x.widget_mut()) {
            child.paint(ctx, data, env);
        }

        // paint the baseline if we're debugging layout
        if env.get(Env::DEBUG_PAINT) && ctx.widget_state.baseline_offset != 0.0 {
            let color = env.get_debug_color(ctx.widget_id().to_raw());
            let my_baseline = ctx.size().height - ctx.widget_state.baseline_offset;
            let line = crate::kurbo::Line::new((0.0, my_baseline), (ctx.size().width, my_baseline));
            let stroke_style = crate::piet::StrokeStyle::new().dash(vec![4.0, 4.0], 0.0);
            ctx.stroke_styled(line, &color, 1.0, &stroke_style);
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
            // in vertical layout, baseline is equivalent to center
            CrossAxisAlignment::Center | CrossAxisAlignment::Baseline => (val / 2.0).round(),
            CrossAxisAlignment::End => val,
            CrossAxisAlignment::Fill => 0.0,
        }
    }
}

struct Spacing {
    alignment: MainAxisAlignment,
    extra: f64,
    n_children: usize,
    index: usize,
    equal_space: f64,
    remainder: f64,
}

impl Spacing {
    /// Given the provided extra space and children count,
    /// this returns an iterator of `f64` spacing,
    /// where the first element is the spacing before any children
    /// and all subsequent elements are the spacing after children.
    fn new(alignment: MainAxisAlignment, extra: f64, n_children: usize) -> Spacing {
        let extra = if extra.is_finite() { extra } else { 0. };
        let equal_space = if n_children > 0 {
            match alignment {
                MainAxisAlignment::Center => extra / 2.,
                MainAxisAlignment::SpaceBetween => extra / (n_children - 1).max(1) as f64,
                MainAxisAlignment::SpaceEvenly => extra / (n_children + 1) as f64,
                MainAxisAlignment::SpaceAround => extra / (2 * n_children) as f64,
                _ => 0.,
            }
        } else {
            0.
        };
        Spacing {
            alignment,
            extra,
            n_children,
            index: 0,
            equal_space,
            remainder: 0.,
        }
    }

    fn next_space(&mut self) -> f64 {
        let desired_space = self.equal_space + self.remainder;
        let actual_space = desired_space.round();
        self.remainder = desired_space - actual_space;
        actual_space
    }
}

impl Iterator for Spacing {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        if self.index > self.n_children {
            return None;
        }
        let result = {
            if self.n_children == 0 {
                self.extra
            } else {
                #[allow(clippy::match_bool)]
                match self.alignment {
                    MainAxisAlignment::Start => match self.index == self.n_children {
                        true => self.extra,
                        false => 0.,
                    },
                    MainAxisAlignment::End => match self.index == 0 {
                        true => self.extra,
                        false => 0.,
                    },
                    MainAxisAlignment::Center => match self.index {
                        0 => self.next_space(),
                        i if i == self.n_children => self.next_space(),
                        _ => 0.,
                    },
                    MainAxisAlignment::SpaceBetween => match self.index {
                        0 => 0.,
                        i if i != self.n_children => self.next_space(),
                        _ => match self.n_children {
                            1 => self.next_space(),
                            _ => 0.,
                        },
                    },
                    MainAxisAlignment::SpaceEvenly => self.next_space(),
                    MainAxisAlignment::SpaceAround => {
                        if self.index == 0 || self.index == self.n_children {
                            self.next_space()
                        } else {
                            self.next_space() + self.next_space()
                        }
                    }
                }
            }
        };
        self.index += 1;
        Some(result)
    }
}

impl From<f64> for FlexParams {
    fn from(flex: f64) -> FlexParams {
        FlexParams {
            flex,
            alignment: None,
        }
    }
}

enum Child<T> {
    Fixed {
        widget: WidgetPod<T, Box<dyn Widget<T>>>,
        alignment: Option<CrossAxisAlignment>,
    },
    Flex {
        widget: WidgetPod<T, Box<dyn Widget<T>>>,
        alignment: Option<CrossAxisAlignment>,
        flex: f64,
    },
    FixedSpacer(KeyOrValue<f64>, f64),
    FlexedSpacer(f64, f64),
}

impl<T> Child<T> {
    fn widget_mut(&mut self) -> Option<&mut WidgetPod<T, Box<dyn Widget<T>>>> {
        match self {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => Some(widget),
            _ => None,
        }
    }
    fn widget(&self) -> Option<&WidgetPod<T, Box<dyn Widget<T>>>> {
        match self {
            Child::Fixed { widget, .. } | Child::Flex { widget, .. } => Some(widget),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_env_log::test;

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_main_axis_alignment_spacing() {
        // The following alignment strategy is based on how
        // Chrome 80 handles it with CSS flex.

        let vec = |a, e, n| -> Vec<f64> { Spacing::new(a, e, n).collect() };

        let a = MainAxisAlignment::Start;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![0., 10.]);
        assert_eq!(vec(a, 10., 2), vec![0., 0., 10.]);
        assert_eq!(vec(a, 10., 3), vec![0., 0., 0., 10.]);

        let a = MainAxisAlignment::End;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![10., 0.]);
        assert_eq!(vec(a, 10., 2), vec![10., 0., 0.]);
        assert_eq!(vec(a, 10., 3), vec![10., 0., 0., 0.]);

        let a = MainAxisAlignment::Center;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![5., 0., 5.]);
        assert_eq!(vec(a, 10., 3), vec![5., 0., 0., 5.]);
        assert_eq!(vec(a, 1., 0), vec![1.]);
        assert_eq!(vec(a, 3., 1), vec![2., 1.]);
        assert_eq!(vec(a, 5., 2), vec![3., 0., 2.]);
        assert_eq!(vec(a, 17., 3), vec![9., 0., 0., 8.]);

        let a = MainAxisAlignment::SpaceBetween;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![0., 10.]);
        assert_eq!(vec(a, 10., 2), vec![0., 10., 0.]);
        assert_eq!(vec(a, 10., 3), vec![0., 5., 5., 0.]);
        assert_eq!(vec(a, 33., 5), vec![0., 8., 9., 8., 8., 0.]);
        assert_eq!(vec(a, 34., 5), vec![0., 9., 8., 9., 8., 0.]);
        assert_eq!(vec(a, 35., 5), vec![0., 9., 9., 8., 9., 0.]);
        assert_eq!(vec(a, 36., 5), vec![0., 9., 9., 9., 9., 0.]);
        assert_eq!(vec(a, 37., 5), vec![0., 9., 10., 9., 9., 0.]);
        assert_eq!(vec(a, 38., 5), vec![0., 10., 9., 10., 9., 0.]);
        assert_eq!(vec(a, 39., 5), vec![0., 10., 10., 9., 10., 0.]);

        let a = MainAxisAlignment::SpaceEvenly;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![3., 4., 3.]);
        assert_eq!(vec(a, 10., 3), vec![3., 2., 3., 2.]);
        assert_eq!(vec(a, 33., 5), vec![6., 5., 6., 5., 6., 5.]);
        assert_eq!(vec(a, 34., 5), vec![6., 5., 6., 6., 5., 6.]);
        assert_eq!(vec(a, 35., 5), vec![6., 6., 5., 6., 6., 6.]);
        assert_eq!(vec(a, 36., 5), vec![6., 6., 6., 6., 6., 6.]);
        assert_eq!(vec(a, 37., 5), vec![6., 6., 7., 6., 6., 6.]);
        assert_eq!(vec(a, 38., 5), vec![6., 7., 6., 6., 7., 6.]);
        assert_eq!(vec(a, 39., 5), vec![7., 6., 7., 6., 7., 6.]);

        let a = MainAxisAlignment::SpaceAround;
        assert_eq!(vec(a, 10., 0), vec![10.]);
        assert_eq!(vec(a, 10., 1), vec![5., 5.]);
        assert_eq!(vec(a, 10., 2), vec![3., 5., 2.]);
        assert_eq!(vec(a, 10., 3), vec![2., 3., 3., 2.]);
        assert_eq!(vec(a, 33., 5), vec![3., 7., 6., 7., 7., 3.]);
        assert_eq!(vec(a, 34., 5), vec![3., 7., 7., 7., 7., 3.]);
        assert_eq!(vec(a, 35., 5), vec![4., 7., 7., 7., 7., 3.]);
        assert_eq!(vec(a, 36., 5), vec![4., 7., 7., 7., 7., 4.]);
        assert_eq!(vec(a, 37., 5), vec![4., 7., 8., 7., 7., 4.]);
        assert_eq!(vec(a, 38., 5), vec![4., 7., 8., 8., 7., 4.]);
        assert_eq!(vec(a, 39., 5), vec![4., 8., 7., 8., 8., 4.]);
    }
}
