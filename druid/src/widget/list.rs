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

//! List view widget.

mod iter;

use std::cmp::Ordering;
use std::f64;
use std::fmt;

pub use self::iter::ListIter;
use crate::kurbo::{Rect, Size};
use crate::widget::{Axis, CrossAxisAlignment};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetPod,
};

/// How to space out elements of the list
#[derive(Debug, Copy, Clone)]
pub enum Spacing {
    /// Space should be flexed.
    Flexed {
        /// The value is only used if `List::flex_items` is true, and if so is the ratio between
        /// the space size and the item size. For example, a value of `2` would mean that spaces
        /// are twice the size of items.
        ratio: f64,
        /// The ratio of the size of end spacing to middle. A value of 0. means no end spacing, 1.
        /// means the same as the middle, and e.g. 0.5 matches `SpaceEvenly` behavor.
        end_ratio: f64,
    },
    /// Spaces should be a fixed width. In this case padding at the start and end of the axis
    /// should be added using e.g. `WidgetExt::padding`.
    Fixed {
        /// The size, in logical pixels, that the space should be.
        size: f64,
    },
}

impl Spacing {
    /// Create a spacing strategy like `SpaceAround`.
    pub fn around(ratio: f64) -> Self {
        Spacing::Flexed {
            ratio,
            end_ratio: 1.0,
        }
    }
    /// Create a spacing strategy like `SpaceEvenly`.
    pub fn evenly(ratio: f64) -> Self {
        Spacing::Flexed {
            ratio,
            end_ratio: 0.5,
        }
    }
    /// Create a spacing strategy like `SpaceBetween`.
    pub fn between(ratio: f64) -> Self {
        Spacing::Flexed {
            ratio,
            end_ratio: 0.0,
        }
    }
    /// Create a spacing strategy with a fixed size spacer.
    pub fn fixed(size: f64) -> Self {
        Spacing::Fixed { size }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Spacing::Fixed { size: 0.0 }
    }
}

/// A list widget for a variable-size collection of items.
pub struct List<T> {
    closure: Box<dyn FnMut() -> Box<dyn Widget<T>>>,
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    axis: Axis,
    /// How to space items
    spacing: Spacing,
    flex_items: bool,
    cross_alignment: CrossAxisAlignment,
}

impl<T> fmt::Debug for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("List")
            .field("axis", &self.axis)
            .field("spacing", &self.spacing)
            .field("flex_items", &self.flex_items)
            .field("cross_alignment", &self.cross_alignment)
            .finish()
    }
}

impl<T: Data> List<T> {
    /// Create a new list widget. Closure will be called every time when a new child
    /// needs to be constructed.
    #[inline]
    pub fn new<W: Widget<T> + 'static>(
        mut closure: impl FnMut() -> W + 'static,
        axis: Axis,
    ) -> Self {
        List {
            closure: Box::new(move || Box::new(closure())),
            children: Vec::new(),
            axis,
            spacing: Spacing::default(),
            flex_items: false,
            cross_alignment: CrossAxisAlignment::Start,
        }
    }

    /// Create a list where items are in a left -> right row
    #[inline]
    pub fn horizontal<W: Widget<T> + 'static>(closure: impl FnMut() -> W + 'static) -> Self {
        Self::new(closure, Axis::Horizontal)
    }

    /// Create a list where items are in a top -> bottom column
    #[inline]
    pub fn vertical<W: Widget<T> + 'static>(closure: impl FnMut() -> W + 'static) -> Self {
        Self::new(closure, Axis::Vertical)
    }

    /// Set the strategy for adding spacing. Defaults to packing the items tightly to the left/top.
    #[inline]
    pub fn with_spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the strategy for adding spacing. Defaults to packing the items tightly to the left/top.
    #[inline]
    pub fn set_spacing(&mut self, spacing: Spacing) -> &mut Self {
        self.spacing = spacing;
        self
    }

    /// Whether items should expand to take up available space.
    #[inline]
    pub fn with_flex_items(mut self, flex_items: bool) -> Self {
        self.flex_items = flex_items;
        self
    }

    /// Whether items should expand to take up available space.
    #[inline]
    pub fn set_flex_items(&mut self, flex_items: bool) -> &mut Self {
        self.flex_items = flex_items;
        self
    }

    /// How to align elements if they are not all the same size in the cross axis.
    #[inline]
    pub fn with_cross_alignment(mut self, cross_alignment: CrossAxisAlignment) -> Self {
        self.cross_alignment = cross_alignment;
        self
    }

    /// How to align elements if they are not all the same size in the cross axis.
    #[inline]
    pub fn set_cross_alignment(&mut self, cross_alignment: CrossAxisAlignment) -> &mut Self {
        self.cross_alignment = cross_alignment;
        self
    }

    /// When the widget is created or the data changes, create or remove children as needed
    ///
    /// Returns `true` if children were added or removed.
    fn update_child_count(&mut self, data: &impl ListIter<T>, _env: &Env) -> bool {
        let len = self.children.len();
        match len.cmp(&data.data_len()) {
            Ordering::Greater => self.children.truncate(data.data_len()),
            Ordering::Less => data.for_each(|_, i| {
                if i >= len {
                    let child = WidgetPod::new((self.closure)());
                    self.children.push(child);
                }
            }),
            Ordering::Equal => (),
        }
        len != data.data_len()
    }
}

impl<C, T> Widget<T> for List<C>
where
    C: Data,
    T: ListIter<C>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        zip_children_mut(&mut self.children, data, |child, data, _| {
            child.event(ctx, event, data, env)
        });
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if self.update_child_count(data, env) {
                ctx.children_changed();
            }
        }

        zip_children(&mut self.children, data, |child, data, _| {
            child.lifecycle(ctx, event, data, env)
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        // we send update to children first, before adding or removing children;
        // this way we avoid sending update to newly added children, at the cost
        // of potentially updating children that are going to be removed.
        zip_children(&mut self.children, data, |child, data, _| {
            child.update(ctx, data, env)
        });

        if self.update_child_count(data, env) {
            ctx.children_changed();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        // TODO quantize to whole pixels (do we need to)
        // TODO cross-axis baseline alignment

        // keep the borrow checker happy
        let axis = self.axis;
        let cross_align = self.cross_alignment;
        let len = self.children.len();

        log::trace!("parameters: {:?}", self);

        // Calculate main axis constraints: it's complicated by spacing.
        let main_constraints = if self.flex_items {
            // Use as little space as possible - if the user wants us to fill more space they
            // should expand the constraints.
            let total = axis.major(bc.min());
            let len = len as f64;
            let size = match self.spacing {
                Spacing::Flexed { ratio, end_ratio } => {
                    total / (len + (len - 1. + 2. * end_ratio) * ratio)
                }
                Spacing::Fixed { size } => (total - (len - 1.) * size) / len,
            };
            (size, size)
        } else {
            (0., f64::INFINITY)
        };
        let item_bc = axis.constraints(bc, main_constraints.0, main_constraints.1);
        log::trace!("Child constraints: {:?}", item_bc);

        // A 2-pass strategy is used: the first pass is to allow children to find their size, and
        // the second pass is then to position everything once we know the children's sizes.

        // Pass 1 - let children calculate their sizes and store it in the child. We also calculate
        // the max size on the cross axis and the total space on the main axis for positioning
        // later.
        let mut cross_max: f64 = 0.0;
        let mut main_total = 0.0;
        zip_children(&mut self.children, data, |child, data, _| {
            let size = child.layout(ctx, &item_bc, data, env);
            // The position is not yet correct - this will be calculated in the second pass, for
            // now set to (0, 0).
            let rect = Rect::ZERO.with_size(size);
            child.set_layout_rect(ctx, data, env, rect);
            // Update counters
            cross_max = cross_max.max(axis.minor(size));
            main_total += axis.major(size);
        });
        log::trace!("cross_max: {}, main_total: {}", cross_max, main_total);

        // Pass 2 - position the children correctly.
        // We need to tell druid the bounds of where our children will paint.
        let mut paint_rect = Rect::ZERO;
        let (spacing, mut main_position) = match self.spacing {
            Spacing::Flexed { ratio, end_ratio } => {
                if self.flex_items {
                    (
                        ratio * main_constraints.1,
                        ratio * main_constraints.1 * end_ratio,
                    )
                } else {
                    // We need to do the middle/ends calculation like for calculating size.
                    if axis.major(bc.max()) < main_total {
                        log::warn!("not enought space to lay out all children");
                    }
                    let min_main = axis.major(bc.min());
                    let spare_space = main_total.max(min_main) - main_total;
                    log::trace!("min_main = {}, spare_space = {}", min_main, spare_space);
                    if spare_space < 1e-6 {
                        (0.0, 0.0)
                    } else {
                        let spacing = spare_space / ((len as f64) - 1. + 2. * end_ratio);
                        (spacing, end_ratio * spacing)
                    }
                }
            }
            Spacing::Fixed { size } => (size, 0.0),
        };
        log::trace!("spacing = {}, main_position = {}", spacing, main_position);

        zip_children(&mut self.children, data, |child, data, _| {
            let size = child.layout_rect().size();
            let cross_position = cross_align.align(cross_max - axis.minor(size));
            // Now we can set the correct position
            let rect = Rect::from_origin_size(axis.pack(main_position, cross_position), size);
            child.set_layout_rect(ctx, data, env, rect);
            // for calculating insets
            paint_rect = paint_rect.union(rect);

            main_position += axis.major(size) + spacing;
        });

        // Correct for end spacing
        let end_ratio = match self.spacing {
            Spacing::Flexed { end_ratio, .. } => end_ratio,
            Spacing::Fixed { .. } => 0.0,
        };
        let main_end = main_position + spacing * (end_ratio - 1.0);
        log::trace!(
            "end_ratio = {}, main_position = {}, spacing = {}, main_end = {}",
            end_ratio,
            main_position,
            spacing,
            main_end
        );

        // Calculate insets and return our size.
        let unconstrained_size: Size = axis.pack(main_end, cross_max).into();
        let size = bc.constrain(unconstrained_size);
        if size != unconstrained_size {
            log::warn!(
                "`List` was constrained from {:?} to {:?}",
                unconstrained_size,
                size
            );
        }
        let insets = paint_rect - Rect::ZERO.with_size(size);
        ctx.set_paint_insets(insets);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        zip_children(&mut self.children, data, |child, data, _| {
            child.paint(ctx, data, env)
        });
    }
}

/// These should disappear once GATs lands (we don't need to use callbacks then).
///
/// The two lists may be of different lengths, and that's ok: we just do the shorter. All the
/// widgets themselves are identical, so adding and removing widgets can happen before or after
/// layout, as long as they aren't included in the algorithm.
fn zip_children<T>(
    children: &mut Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    children_data: &impl ListIter<T>,
    mut cb: impl FnMut(&mut WidgetPod<T, Box<dyn Widget<T>>>, &T, usize),
) {
    let mut children = children.iter_mut();
    children_data.for_each(|data, idx| {
        let child = match children.next() {
            Some(child) => child,
            None => {
                return;
            }
        };
        cb(child, data, idx)
    });
}

fn zip_children_mut<T>(
    children: &mut Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    children_data: &mut impl ListIter<T>,
    mut cb: impl FnMut(&mut WidgetPod<T, Box<dyn Widget<T>>>, &mut T, usize),
) {
    let mut children = children.iter_mut();
    children_data.for_each_mut(|data, idx| {
        let child = match children.next() {
            Some(child) => child,
            None => {
                return;
            }
        };
        cb(child, data, idx)
    });
}
