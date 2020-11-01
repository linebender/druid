// Copyright 2020 The Druid Authors.
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

//! Widget that displays a long collection of items without keeping all item
//! widgets in memory.

use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

#[cfg(feature = "im")]
use crate::im::Vector;

use crate::kurbo::{Point, Rect, Size, Vec2};

use crate::scroll_component::ScrollComponent;
use crate::widget::ClipBox;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Selector, UpdateCtx, Widget, WidgetPod,
};

/// A virtualized scrolling widget for a (possibly large) collection of items.
pub struct VirtList<C, T> {
    clip: ClipBox<C, VirtListInner<T>>,
    scroll_component: ScrollComponent,
}

impl<C: RangeIter<T>, T: Data> VirtList<C, T> {
    /// Create a new vertical list widget. Closure will be called every time when a new child
    /// needs to be constructed. Child height needs to be constant.
    pub fn vertical<W: Widget<T> + 'static>(
        child_height: f64,
        closure: impl Fn() -> W + 'static,
    ) -> Self {
        let inner = VirtListInner::new(
            Size::new(0.0, child_height),
            Box::new(move || Box::new(closure())),
        );
        Self {
            clip: ClipBox::new(inner)
                .constrain_vertical(false)
                .constrain_horizontal(true),
            scroll_component: ScrollComponent::new(),
        }
    }
}

impl<C: RangeIter<T>, T: Data> Widget<C> for VirtList<C, T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut C, env: &Env) {
        let scroll_component = &mut self.scroll_component;
        self.clip.with_port(|port| {
            scroll_component.event(port, ctx, event, env);
        });

        // scroll_component has modified the viewport, report the offset to the inner list
        let offset = self.clip.viewport_origin().to_vec2();
        let needs_update = self.clip.child_mut().set_viewport_offset(offset);
        if needs_update {
            // The item offset has changed, we need to schedule an update to change visible
            // data in item widgets.
            ctx.request_update_child(self.clip.child_pod_mut());
        }

        if !ctx.is_handled() {
            self.clip.event(ctx, event, data, env);
        }

        self.clip.with_port(|port| {
            scroll_component.handle_scroll(port, ctx, event, env);
        });
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &C, env: &Env) {
        if let LifeCycle::Size(_) = &event {
            // The size of the viewport has changed, `VirtListInner` will take care of
            // adding/removing children.
            let size = self.clip.viewport_size();
            let needs_update = self.clip.child_mut().set_viewport_size(size);
            if needs_update {
                let child_id = self.clip.child_pod().id();
                ctx.submit_command(VIEWPORT_SIZE_CHANGED.to(child_id));
            }
        }
        self.scroll_component.lifecycle(ctx, event, env);
        self.clip.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &C, data: &C, env: &Env) {
        self.clip.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &C, env: &Env) -> Size {
        bc.debug_check("VirtList");

        let old_size = self.clip.viewport().rect.size();
        let child_size = self.clip.layout(ctx, &bc, data, env);

        let self_size = bc.constrain(child_size);
        // The new size might have made the current scroll offset invalid. This makes it valid
        // again.
        let _ = self.clip.pan_by(Vec2::ZERO);
        if old_size != self_size {
            self.scroll_component
                .reset_scrollbar_fade(|d| ctx.request_timer(d), env);
        }

        self_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &C, env: &Env) {
        self.clip.paint(ctx, data, env);
        self.scroll_component
            .draw_bars(ctx, &self.clip.viewport(), env);
    }
}

struct VirtListInner<T> {
    child_size: Size,
    closure: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    visible_count: usize,
    data_offset: usize,
}

impl<T> VirtListInner<T> {
    fn new(child_size: Size, closure: Box<dyn Fn() -> Box<dyn Widget<T>>>) -> Self {
        Self {
            child_size,
            closure,
            children: Vec::new(),
            visible_count: 0,
            data_offset: 0,
        }
    }

    /// Update the size of the viewport. Returns `true` if the number of visible children
    /// changed and `self` should get an update scheduled.
    fn set_viewport_size(&mut self, size: Size) -> bool {
        let visible_children = (size.height / self.child_size.height).ceil() as usize + 1;
        if self.visible_count != visible_children {
            self.visible_count = visible_children;
            true
        } else {
            false
        }
    }

    /// Update the current scrolling offset. Returns `true` if the item offset changed
    /// and `self` should get an update scheduled.
    fn set_viewport_offset(&mut self, offset: Vec2) -> bool {
        let children_above_offset = (offset.y / self.child_size.height).floor() as usize;
        if self.data_offset != children_above_offset {
            self.data_offset = children_above_offset;
            true
        } else {
            false
        }
    }

    /// When the widget is created, the data changes, or size of the viewport is changed,
    /// add or remove widgets for visible children.
    ///
    /// Returns `true` if children were added or removed.
    fn update_child_count(&mut self, data: &impl RangeIter<T>) -> bool {
        let current = self.children.len();
        let needed = self.visible_count.min(data.data_len());
        match current.cmp(&needed) {
            Ordering::Greater => {
                self.children.truncate(needed);
                true
            }
            Ordering::Less => {
                for _ in current..needed {
                    let child = WidgetPod::new((self.closure)());
                    self.children.push(child);
                }
                true
            }
            Ordering::Equal => false,
        }
    }

    fn visible_data_range(&self, data: &impl RangeIter<T>) -> Range<usize> {
        let from = self.data_offset;
        let to = self.data_offset + self.children.len();
        from.min(data.data_len())..to.min(data.data_len())
    }

    fn child(&mut self, i: usize) -> &mut WidgetPod<T, Box<dyn Widget<T>>> {
        let len = self.children.len();
        &mut self.children[i % len]
    }
}

const VIEWPORT_SIZE_CHANGED: Selector =
    Selector::new("druid-builtin.virtlist.viewport-size-changed");

impl<C: RangeIter<T>, T: Data> Widget<C> for VirtListInner<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut C, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(VIEWPORT_SIZE_CHANGED) => {
                if self.update_child_count(data) {
                    ctx.children_changed();
                }
            }
            _ => {
                data.for_in_mut(self.visible_data_range(data), |child_data, i| {
                    self.child(i).event(ctx, event, child_data, env);
                });
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &C, env: &Env) {
        if let LifeCycle::WidgetAdded = &event {
            if self.update_child_count(data) {
                ctx.children_changed();
            }
        }
        data.for_in(self.visible_data_range(data), |child_data, i| {
            self.child(i).lifecycle(ctx, event, child_data, env);
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &C, data: &C, env: &Env) {
        data.for_in(self.visible_data_range(data), |child_data, i| {
            self.child(i).update(ctx, child_data, env);
        });
        if self.update_child_count(data) {
            ctx.children_changed();
        } else if old_data.data_len() != data.data_len() {
            // The number of visible children haven't changed, but the total number
            // of rows did, and therefore our total size.  We need to request layout.
            ctx.request_layout();
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &C, env: &Env) -> Size {
        let range = self.visible_data_range(data);
        let height_above = self.child_size.height * range.start as f64;
        let height_below = self.child_size.height * (data.data_len() - range.end) as f64;

        let mut paint_rect = Rect::ZERO;
        let mut width = bc.min().width;
        let mut y = 0.0;

        y += height_above;
        data.for_in(range, |child_data, i| {
            let child = self.child(i);
            let child_bc = BoxConstraints::new(
                Size::new(bc.min().width, 0.0),
                Size::new(bc.max().width, std::f64::INFINITY),
            );
            let child_size = child.layout(ctx, &child_bc, child_data, env);
            let rect = Rect::from_origin_size(Point::new(0.0, y), child_size);
            child.set_layout_rect(ctx, child_data, env, rect);
            paint_rect = paint_rect.union(child.paint_rect());
            width = width.max(child_size.width);
            y += child_size.height;
        });
        y += height_below;

        let my_size = bc.constrain(Size::new(width, y));
        let insets = paint_rect - my_size.to_rect();
        ctx.set_paint_insets(insets);
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &C, env: &Env) {
        data.for_in(self.visible_data_range(data), |child_data, i| {
            self.child(i).paint(ctx, child_data, env);
        });
    }
}

/// This iterator-like trait enables using `VirtList` with any kind of `Data`,
/// with efficient iteration of sub-ranges.
pub trait RangeIter<T>: Data {
    /// Iterate over each child in given range. Panic if the range is out of bounds.
    fn for_in(&self, r: Range<usize>, cb: impl FnMut(&T, usize));

    /// Iterate over each child in given range and update self in case of changed data.
    /// Panic if the range is out of bounds.
    fn for_in_mut(&mut self, r: Range<usize>, cb: impl FnMut(&mut T, usize));

    /// Return total data length.
    fn data_len(&self) -> usize;
}

#[cfg(feature = "im")]
impl<T: Data> RangeIter<T> for Vector<T> {
    fn for_in(&self, r: Range<usize>, mut cb: impl FnMut(&T, usize)) {
        if r.is_empty() {
            return;
        }
        let offset = r.start;
        for (i, item) in self.focus().narrow(r).into_iter().enumerate() {
            cb(item, offset + i);
        }
    }

    fn for_in_mut(&mut self, r: Range<usize>, mut cb: impl FnMut(&mut T, usize)) {
        if r.is_empty() {
            return;
        }
        let offset = r.start;
        for (i, item) in self.focus_mut().narrow(r).into_iter().enumerate() {
            cb(item, offset + i);
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

// S == shared data type
#[cfg(feature = "im")]
impl<S: Data, T: Data> RangeIter<(S, T)> for (S, Vector<T>) {
    fn for_in(&self, r: Range<usize>, mut cb: impl FnMut(&(S, T), usize)) {
        if r.is_empty() {
            return;
        }
        let offset = r.start;
        for (i, item) in self.1.focus().narrow(r).into_iter().enumerate() {
            let d = (self.0.to_owned(), item.to_owned());
            cb(&d, offset + i);
        }
    }

    fn for_in_mut(&mut self, r: Range<usize>, mut cb: impl FnMut(&mut (S, T), usize)) {
        if r.is_empty() {
            return;
        }
        let offset = r.start;
        for (i, item) in self.1.focus_mut().narrow(r).into_iter().enumerate() {
            let mut d = (self.0.clone(), item.clone());
            cb(&mut d, offset + i);

            if !self.0.same(&d.0) {
                self.0 = d.0;
            }
            if !item.same(&d.1) {
                *item = d.1;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.1.len()
    }
}

impl<T: Data> RangeIter<T> for Arc<Vec<T>> {
    fn for_in(&self, r: Range<usize>, mut cb: impl FnMut(&T, usize)) {
        let offset = r.start;
        for (i, item) in self[r].iter().enumerate() {
            cb(item, offset + i);
        }
    }

    fn for_in_mut(&mut self, r: Range<usize>, mut cb: impl FnMut(&mut T, usize)) {
        let offset = r.start;
        let mut new_data = Vec::with_capacity(r.end - r.start);
        let mut any_changed = false;

        for (i, item) in self[r].iter().enumerate() {
            let mut d = item.to_owned();
            cb(&mut d, offset + i);

            if !any_changed && !item.same(&d) {
                any_changed = true;
            }
            new_data.push(d);
        }

        if any_changed {
            let mut cloned = Vec::clone(self);
            for (i, item) in new_data.into_iter().enumerate() {
                cloned[offset + i] = item;
            }
            *self = Arc::new(cloned);
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}
