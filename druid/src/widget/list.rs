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

pub use self::iter::ListIter;
#[cfg(feature = "im")]
use crate::im::Vector;
use crate::kurbo::{common::FloatExt, Point, Rect, Size};
use crate::widget::{flex::Spacing, Axis, CrossAxisAlignment, MainAxisAlignment};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetPod,
};

#[derive(Copy, Clone)]
pub enum ListMainAlignment {
    /// Inherit options from `MainAxisAlignment`.
    NoFlex(MainAxisAlignment),
    /// Force children to use up all space evenly.
    ///
    /// Spacing will be added between items, but not at ends. If you don't want spacing set it to
    /// `0.0`.
    FlexItems { spacing: f64 },
}

impl From<MainAxisAlignment> for ListMainAlignment {
    fn from(from: MainAxisAlignment) -> Self {
        ListMainAlignment::NoFlex(from)
    }
}

/// A list widget for a variable-size collection of items.
pub struct List<T> {
    closure: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    axis: Axis,
    main_align: ListMainAlignment,
    cross_align: CrossAxisAlignment,
}

impl<T: Data> List<T> {
    /// Create a new list widget. Closure will be called every time when a new child
    /// needs to be constructed.
    #[inline]
    pub fn new<W: Widget<T> + 'static>(closure: impl Fn() -> W + 'static, axis: Axis) -> Self {
        List {
            closure: Box::new(move || Box::new(closure())),
            children: Vec::new(),
            axis,
            main_align: MainAxisAlignment::Start.into(),
            cross_align: CrossAxisAlignment::Start,
        }
    }

    /// Create a list where items are in a left -> right row
    #[inline]
    pub fn horizontal<W: Widget<T> + 'static>(closure: impl Fn() -> W + 'static) -> Self {
        Self::new(closure, Axis::Horizontal)
    }

    /// Create a list where items are in a top -> bottom column
    #[inline]
    pub fn vertical<W: Widget<T> + 'static>(closure: impl Fn() -> W + 'static) -> Self {
        Self::new(closure, Axis::Vertical)
    }

    /// If set to `true`, each element will be given an equal share of the space available.
    ///
    /// Can pass either a `ListMainAlignment` or a `MainAxisAlignment`.
    #[inline]
    pub fn with_main_alignment(mut self, main_align: impl Into<ListMainAlignment>) -> Self {
        self.main_align = main_align.into();
        self
    }

    /// If set to `true`, each element will be given an equal share of the space available.
    ///
    /// Can pass either a `ListMainAlignment` or a `MainAxisAlignment`.
    #[inline]
    pub fn set_main_alignment(&mut self, main_align: impl Into<ListMainAlignment>) -> &mut Self {
        self.main_align = main_align.into();
        self
    }

    /// If non-zero, then spacing will be added between elements.
    #[inline]
    pub fn with_cross_alignment(mut self, cross_align: CrossAxisAlignment) -> Self {
        self.cross_align = cross_align;
        self
    }

    /// If non-zero, then spacing will be added between elements.
    #[inline]
    pub fn set_cross_alignment(&mut self, cross_align: CrossAxisAlignment) -> &mut Self {
        self.cross_align = cross_align;
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
        let mut children = self.children.iter_mut();
        data.for_each_mut(|child_data, _| {
            if let Some(child) = children.next() {
                child.event(ctx, event, child_data, env);
            }
        });
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if self.update_child_count(data, env) {
                ctx.children_changed();
            }
        }

        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.lifecycle(ctx, event, child_data, env);
            }
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        // we send update to children first, before adding or removing children;
        // this way we avoid sending update to newly added children, at the cost
        // of potentially updating children that are going to be removed.
        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.update(ctx, child_data, env);
            }
        });

        if self.update_child_count(data, env) {
            ctx.children_changed();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        // keep the borrow checker happy
        let axis = self.axis;
        let cross_align = self.cross_align;

        let mut minor = axis.minor(bc.min());
        let mut major = 0.0;

        let mut paint_rect = Rect::ZERO;
        let mut children = self.children.iter_mut();

        // TODO quantize to whole pixels (do we need to)
        // major constraint
        let maj_c = match self.main_align {
            // This is the only layout where children are constriained on the major axis.
            ListMainAlignment::FlexItems { spacing } => {
                let len = data.data_len() as f64;
                // We need to take spacing into account when working out size, but we need to
                // multiply by `(n - 1) / n` to fix the fence/fencepost problem.
                let div = (axis.major(bc.max()) - spacing * (len - 1.)) / len;
                (div, div)
            }
            _ => (0., f64::INFINITY),
        };
        // TODO cross axis alignment

        // We branch on flex, because the flex option only requires 1 pass - we don't need to
        // find out the sizes of the children because we will enforce size.
        match self.main_align {
            // 1 pass to layout items.
            ListMainAlignment::FlexItems { spacing } => {
                data.for_each(|child_data, _| {
                    let child = match children.next() {
                        Some(child) => child,
                        None => {
                            return;
                        }
                    };
                    let child_bc = axis.constraints(bc, maj_c.0, maj_c.1);
                    let child_size = child.layout(ctx, &child_bc, child_data, env);
                    let rect =
                        Rect::from_origin_size(Point::from(axis.pack(major, 0.0)), child_size);
                    child.set_layout_rect(ctx, child_data, env, rect);
                    paint_rect = paint_rect.union(child.paint_rect());
                    minor = minor.max(axis.minor(child_size));
                    major += axis.major(child_size) + spacing;
                });

                // Correct for overshoot
                major -= spacing;

                let unconstrained_size: Size = axis.pack(major, minor).into();
                let my_size = bc.constrain(unconstrained_size);
                let insets = paint_rect - Rect::ZERO.with_size(my_size);
                ctx.set_paint_insets(insets);
                my_size
            }
            ListMainAlignment::NoFlex(main_align) => {
                // Measure children.
                let mut major_non_flex = 0.0;
                let mut children = self.children.iter_mut();
                data.for_each(|child_data, _| {
                    let child = match children.next() {
                        Some(child) => child,
                        None => {
                            return;
                        }
                    };
                    let child_bc = axis.constraints(bc, maj_c.0, maj_c.1);
                    let child_size = child.layout(ctx, &child_bc, child_data, env);

                    if child_size.width.is_infinite() {
                        log::warn!("A non-Flex child has an infinite width.");
                    }

                    if child_size.height.is_infinite() {
                        log::warn!("A non-Flex child has an infinite height.");
                    }

                    major_non_flex += axis.major(child_size).expand();
                    minor = minor.max(axis.minor(child_size).expand());
                    // Stash size.
                    let rect = child_size.to_rect();
                    child.set_layout_rect(ctx, child_data, env, rect);
                });

                let total_major = axis.major(bc.max());
                let extra = (total_major - major_non_flex).max(0.0);
                let mut spacing = Spacing::new(main_align, extra, self.children.len());

                // Lay out the children.
                let mut major = spacing.next().unwrap_or(0.);
                let mut child_paint_rect = Rect::ZERO;
                let mut children = self.children.iter_mut();
                data.for_each(|child_data, _| {
                    let child = match children.next() {
                        Some(child) => child,
                        None => {
                            return;
                        }
                    };
                    let child_size = child.layout_rect().size();
                    let child_minor_offset = {
                        let extra_minor = minor - axis.minor(child_size);
                        cross_align.align(extra_minor)
                    };

                    let child_pos: Point = axis.pack(major, child_minor_offset).into();
                    let child_frame = Rect::from_origin_size(child_pos, child_size);
                    child.set_layout_rect(ctx, child_data, env, child_frame);
                    child_paint_rect = child_paint_rect.union(child.paint_rect());
                    major += axis.major(child_size).expand();
                    major += spacing.next().unwrap_or(0.);
                });

                let my_size: Size = axis.pack(major, minor).into();
                let max_major = axis.major(bc.max());
                let my_size = axis.constraints(bc, 0.0, max_major).constrain(my_size);

                let my_bounds = Rect::ZERO.with_size(my_size);
                let insets = child_paint_rect - my_bounds;
                ctx.set_paint_insets(insets);
                my_size
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.paint(ctx, child_data, env);
            }
        });
    }
}
