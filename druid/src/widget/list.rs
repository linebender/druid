// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Simple list view widget.

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::f64;
use std::ops::Deref;
use std::sync::Arc;

use tracing::{instrument, trace};

#[cfg(feature = "im")]
use crate::im::{OrdMap, Vector};

use crate::kurbo::{Point, Rect, Size};

use crate::debug_state::DebugState;
use crate::{
    widget::Axis, BoxConstraints, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, UpdateCtx, Widget, WidgetPod,
};

/// A list widget for a variable-size collection of items.
pub struct List<T> {
    closure: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    children: Vec<WidgetPod<T, Box<dyn Widget<T>>>>,
    axis: Axis,
    spacing: KeyOrValue<f64>,
    old_bc: BoxConstraints,
}

impl<T: Data> List<T> {
    /// Create a new list widget. Closure will be called every time when a new child
    /// needs to be constructed.
    pub fn new<W: Widget<T> + 'static>(closure: impl Fn() -> W + 'static) -> Self {
        List {
            closure: Box::new(move || Box::new(closure())),
            children: Vec::new(),
            axis: Axis::Vertical,
            spacing: KeyOrValue::Concrete(0.),
            old_bc: BoxConstraints::tight(Size::ZERO),
        }
    }

    /// Sets the widget to display the list horizontally, not vertically.
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }

    /// Set the spacing between elements.
    pub fn with_spacing(mut self, spacing: impl Into<KeyOrValue<f64>>) -> Self {
        self.spacing = spacing.into();
        self
    }

    /// Set the spacing between elements.
    pub fn set_spacing(&mut self, spacing: impl Into<KeyOrValue<f64>>) -> &mut Self {
        self.spacing = spacing.into();
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

/// This iterator enables writing List widget for any `Data`.
pub trait ListIter<T>: Data {
    /// Iterate over each data child.
    fn for_each(&self, cb: impl FnMut(&T, usize));

    /// Iterate over each data child. Keep track of changed data and update self.
    fn for_each_mut(&mut self, cb: impl FnMut(&mut T, usize));

    /// Return data length.
    fn data_len(&self) -> usize;
}
#[cfg(feature = "im")]
impl<T: Data> ListIter<T> for Vector<T> {
    fn for_each(&self, mut cb: impl FnMut(&T, usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(item, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut T, usize)) {
        for (index, element) in self.clone().iter().enumerate() {
            let mut new_element = element.to_owned();
            cb(&mut new_element, index);
            if !new_element.same(element) {
                self[index] = new_element;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

//An implementation for ListIter<(K, V)> has been omitted due to problems
//with how the List Widget handles the reordering of its data.
#[cfg(feature = "im")]
impl<K, V> ListIter<V> for OrdMap<K, V>
where
    K: Data + Ord,
    V: Data,
{
    fn for_each(&self, mut cb: impl FnMut(&V, usize)) {
        for (i, item) in self.iter().enumerate() {
            let ret = (item.0.to_owned(), item.1.to_owned());
            cb(&ret.1, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut V, usize)) {
        for (i, item) in self.clone().iter().enumerate() {
            let mut ret = item.1.clone();
            cb(&mut ret, i);

            if !item.1.same(&ret) {
                self[item.0] = ret;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

// S == shared data type
#[cfg(feature = "im")]
impl<S: Data, T: Data> ListIter<(S, T)> for (S, Vector<T>) {
    fn for_each(&self, mut cb: impl FnMut(&(S, T), usize)) {
        for (i, item) in self.1.iter().enumerate() {
            let d = (self.0.to_owned(), item.to_owned());
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (S, T), usize)) {
        for (index, element) in self.1.clone().iter().enumerate() {
            let mut d = (self.0.clone(), element.to_owned());
            cb(&mut d, index);

            if !self.0.same(&d.0) {
                self.0 = d.0;
            }
            if !element.same(&d.1) {
                self.1[index] = d.1;
            }
        }
    }

    fn data_len(&self) -> usize {
        self.1.len()
    }
}

impl<T: Data> ListIter<T> for Arc<Vec<T>> {
    fn for_each(&self, mut cb: impl FnMut(&T, usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(item, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut T, usize)) {
        let mut new_data: Option<Vec<T>> = None;

        for (i, item) in self.iter().enumerate() {
            let mut d = item.to_owned();
            cb(&mut d, i);

            if !item.same(&d) {
                match &mut new_data {
                    Some(vec) => {
                        vec[i] = d;
                    }
                    None => {
                        let mut new = (**self).clone();
                        new[i] = d;
                        new_data = Some(new);
                    }
                }
            }
        }
        if let Some(vec) = new_data {
            *self = Arc::new(vec);
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

// S == shared data type
impl<S: Data, T: Data> ListIter<(S, T)> for (S, Arc<Vec<T>>) {
    fn for_each(&self, mut cb: impl FnMut(&(S, T), usize)) {
        for (i, item) in self.1.iter().enumerate() {
            let d = (self.0.clone(), item.to_owned());
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (S, T), usize)) {
        let mut new_data: Option<Vec<T>> = None;

        for (i, item) in self.1.iter().enumerate() {
            let mut d = (self.0.clone(), item.to_owned());
            cb(&mut d, i);

            self.0 = d.0;

            if !item.same(&d.1) {
                match &mut new_data {
                    Some(vec) => {
                        vec[i] = d.1;
                    }
                    None => {
                        let mut new = self.1.deref().clone();
                        new[i] = d.1;
                        new_data = Some(new);
                    }
                }
            }
        }
        if let Some(vec) = new_data {
            self.1 = Arc::new(vec);
        }
    }

    fn data_len(&self) -> usize {
        self.1.len()
    }
}

impl<T: Data> ListIter<T> for Arc<VecDeque<T>> {
    fn for_each(&self, mut cb: impl FnMut(&T, usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(item, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut T, usize)) {
        let mut new_data: Option<VecDeque<T>> = None;

        for (i, item) in self.iter().enumerate() {
            let mut d = item.to_owned();
            cb(&mut d, i);

            if !item.same(&d) {
                match &mut new_data {
                    Some(vec) => {
                        vec[i] = d;
                    }
                    None => {
                        let mut new = (**self).clone();
                        new[i] = d;
                        new_data = Some(new);
                    }
                }
            }
        }
        if let Some(vec) = new_data {
            *self = Arc::new(vec);
        }
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

// S == shared data type
impl<S: Data, T: Data> ListIter<(S, T)> for (S, Arc<VecDeque<T>>) {
    fn for_each(&self, mut cb: impl FnMut(&(S, T), usize)) {
        for (i, item) in self.1.iter().enumerate() {
            let d = (self.0.clone(), item.to_owned());
            cb(&d, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (S, T), usize)) {
        let mut new_data: Option<VecDeque<T>> = None;

        for (i, item) in self.1.iter().enumerate() {
            let mut d = (self.0.clone(), item.to_owned());
            cb(&mut d, i);

            self.0 = d.0;

            if !item.same(&d.1) {
                match &mut new_data {
                    Some(vec) => {
                        vec[i] = d.1;
                    }
                    None => {
                        let mut new = self.1.deref().clone();
                        new[i] = d.1;
                        new_data = Some(new);
                    }
                }
            }
        }
        if let Some(vec) = new_data {
            self.1 = Arc::new(vec);
        }
    }

    fn data_len(&self) -> usize {
        self.1.len()
    }
}

impl<C: Data, T: ListIter<C>> Widget<T> for List<C> {
    #[instrument(name = "List", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut children = self.children.iter_mut();
        data.for_each_mut(|child_data, _| {
            if let Some(child) = children.next() {
                child.event(ctx, event, child_data, env);
            }
        });
    }

    #[instrument(name = "List", level = "trace", skip(self, ctx, event, data, env))]
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

    #[instrument(name = "List", level = "trace", skip(self, ctx, _old_data, data, env))]
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

        if ctx.env_key_changed(&self.spacing) {
            ctx.request_layout();
        }
    }

    #[instrument(name = "List", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let axis = self.axis;
        let spacing = self.spacing.resolve(env);
        let mut minor = axis.minor(bc.min());
        let mut major_pos = 0.0;
        let mut paint_rect = Rect::ZERO;

        let bc_changed = self.old_bc != *bc;
        self.old_bc = *bc;

        let mut children = self.children.iter_mut();
        let child_bc = axis.constraints(bc, 0., f64::INFINITY);
        data.for_each(|child_data, _| {
            let child = match children.next() {
                Some(child) => child,
                None => {
                    return;
                }
            };

            let child_size = if bc_changed || child.layout_requested() {
                child.layout(ctx, &child_bc, child_data, env)
            } else {
                child.layout_rect().size()
            };

            let child_pos: Point = axis.pack(major_pos, 0.).into();
            child.set_origin(ctx, child_pos);
            paint_rect = paint_rect.union(child.paint_rect());
            minor = minor.max(axis.minor(child_size));
            major_pos += axis.major(child_size) + spacing;
        });

        // correct overshoot at end.
        major_pos -= spacing;

        let my_size = bc.constrain(Size::from(axis.pack(major_pos, minor)));
        let insets = paint_rect - my_size.to_rect();
        ctx.set_paint_insets(insets);
        trace!("Computed layout: size={}, insets={:?}", my_size, insets);
        my_size
    }

    #[instrument(name = "List", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut children = self.children.iter_mut();
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                child.paint(ctx, child_data, env);
            }
        });
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let mut children = self.children.iter();
        let mut children_state = Vec::with_capacity(data.data_len());
        data.for_each(|child_data, _| {
            if let Some(child) = children.next() {
                children_state.push(child.widget().debug_state(child_data));
            }
        });

        DebugState {
            display_name: "List".to_string(),
            children: children_state,
            ..Default::default()
        }
    }
}
