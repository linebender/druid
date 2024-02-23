// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Helper types for test writing.
//!
//! This includes tools for making throwaway widgets more easily.
//!
//! Note: Some of these types are undocumented. They're meant to help maintainers of Druid and
//! people trying to build a framework on top of Druid (like crochet), not to be user-facing.

#![allow(missing_docs)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::*;

pub type EventFn<S, T> = dyn FnMut(&mut S, &mut EventCtx, &Event, &mut T, &Env);
pub type LifeCycleFn<S, T> = dyn FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle, &T, &Env);
pub type UpdateFn<S, T> = dyn FnMut(&mut S, &mut UpdateCtx, &T, &T, &Env);
pub type LayoutFn<S, T> = dyn FnMut(&mut S, &mut LayoutCtx, &BoxConstraints, &T, &Env) -> Size;
pub type PaintFn<S, T> = dyn FnMut(&mut S, &mut PaintCtx, &T, &Env);

pub const REPLACE_CHILD: Selector = Selector::new("druid-test.replace-child");

/// A widget that can be constructed from individual functions, builder-style.
///
/// This widget is generic over its state, which is passed in at construction time.
pub struct ModularWidget<S, T> {
    state: S,
    event: Option<Box<EventFn<S, T>>>,
    lifecycle: Option<Box<LifeCycleFn<S, T>>>,
    update: Option<Box<UpdateFn<S, T>>>,
    layout: Option<Box<LayoutFn<S, T>>>,
    paint: Option<Box<PaintFn<S, T>>>,
}

/// A widget that can replace its child on command
pub struct ReplaceChild<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    replacer: Box<dyn Fn() -> Box<dyn Widget<T>>>,
}

/// A widget that records each time one of its methods is called.
///
/// Make one like this:
///
/// ```
/// # use druid::widget::Label;
/// # use druid::{WidgetExt, LifeCycle};
/// use druid::tests::helpers::{Recording, Record, TestWidgetExt};
/// use druid::tests::harness::Harness;
/// let recording = Recording::default();
/// let widget = Label::new("Hello").padding(4.0).record(&recording);
///
/// Harness::create_simple((), widget, |harness| {
///     harness.send_initial_events();
///     assert!(matches!(recording.next(), Record::L(LifeCycle::WidgetAdded)));
/// })
/// ```
pub struct Recorder<W> {
    recording: Recording,
    child: W,
}

/// A recording of widget method calls.
#[derive(Debug, Clone, Default)]
pub struct Recording(Rc<RefCell<VecDeque<Record>>>);

/// A recording of a method call on a widget.
///
/// Each member of the enum corresponds to one of the methods on `Widget`.
#[derive(Debug, Clone)]
pub enum Record {
    /// An `Event`.
    E(Event),
    /// A `LifeCycle` event.
    L(LifeCycle),
    Layout(Size),
    Update(Region),
    Paint,
    // instead of always returning an Option<Record>, we have a none variant;
    // this would be code smell elsewhere but here I think it makes the tests
    // easier to read.
    None,
}

/// like WidgetExt but just for this one thing
pub trait TestWidgetExt<T: Data>: Widget<T> + Sized + 'static {
    fn record(self, recording: &Recording) -> Recorder<Self> {
        Recorder {
            child: self,
            recording: recording.clone(),
        }
    }
}

impl<T: Data, W: Widget<T> + 'static> TestWidgetExt<T> for W {}

#[allow(dead_code)]
impl<S, T> ModularWidget<S, T> {
    pub fn new(state: S) -> Self {
        ModularWidget {
            state,
            event: None,
            lifecycle: None,
            update: None,
            layout: None,
            paint: None,
        }
    }

    pub fn event_fn(
        mut self,
        f: impl FnMut(&mut S, &mut EventCtx, &Event, &mut T, &Env) + 'static,
    ) -> Self {
        self.event = Some(Box::new(f));
        self
    }

    pub fn lifecycle_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LifeCycleCtx, &LifeCycle, &T, &Env) + 'static,
    ) -> Self {
        self.lifecycle = Some(Box::new(f));
        self
    }

    pub fn update_fn(
        mut self,
        f: impl FnMut(&mut S, &mut UpdateCtx, &T, &T, &Env) + 'static,
    ) -> Self {
        self.update = Some(Box::new(f));
        self
    }

    pub fn layout_fn(
        mut self,
        f: impl FnMut(&mut S, &mut LayoutCtx, &BoxConstraints, &T, &Env) -> Size + 'static,
    ) -> Self {
        self.layout = Some(Box::new(f));
        self
    }

    pub fn paint_fn(mut self, f: impl FnMut(&mut S, &mut PaintCtx, &T, &Env) + 'static) -> Self {
        self.paint = Some(Box::new(f));
        self
    }
}

impl<S, T: Data> Widget<T> for ModularWidget<S, T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(f) = self.event.as_mut() {
            f(&mut self.state, ctx, event, data, env)
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let Some(f) = self.lifecycle.as_mut() {
            f(&mut self.state, ctx, event, data, env)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(f) = self.update.as_mut() {
            f(&mut self.state, ctx, old_data, data, env)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let ModularWidget {
            ref mut state,
            ref mut layout,
            ..
        } = self;
        layout
            .as_mut()
            .map(|f| f(state, ctx, bc, data, env))
            .unwrap_or_else(|| Size::new(100., 100.))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(f) = self.paint.as_mut() {
            f(&mut self.state, ctx, data, env)
        }
    }
}

impl<T: Data> ReplaceChild<T> {
    pub fn new<W: Widget<T> + 'static>(
        child: impl Widget<T> + 'static,
        f: impl Fn() -> W + 'static,
    ) -> Self {
        let child = WidgetPod::new(child.boxed());
        let replacer = Box::new(move || f().boxed());
        ReplaceChild { child, replacer }
    }
}

impl<T: Data> Widget<T> for ReplaceChild<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Command(cmd) = event {
            if cmd.is(REPLACE_CHILD) {
                self.child = WidgetPod::new((self.replacer)());
                ctx.children_changed();
                return;
            }
        }
        self.child.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.child.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint_raw(ctx, data, env)
    }
}

#[allow(dead_code)]
impl Recording {
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear()
    }

    /// Returns the next event in the recording, if one exists.
    ///
    /// This consumes the event.
    pub fn next(&self) -> Record {
        self.0.borrow_mut().pop_front().unwrap_or(Record::None)
    }

    /// Returns an iterator of events drained from the recording.
    pub fn drain(&self) -> impl Iterator<Item = Record> {
        self.0
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn push(&self, event: Record) {
        self.0.borrow_mut().push_back(event)
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Recorder<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.recording.push(Record::E(event.clone()));
        self.child.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let should_record = !matches!(
            event,
            LifeCycle::Internal(InternalLifeCycle::DebugRequestState { .. })
                | LifeCycle::Internal(InternalLifeCycle::DebugInspectState(_))
        );

        if should_record {
            self.recording.push(Record::L(event.clone()));
        }

        self.child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
        self.recording
            .push(Record::Update(ctx.widget_state.invalid.clone()));
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        self.recording.push(Record::Layout(size));
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
        self.recording.push(Record::Paint)
    }
}

pub fn widget_ids<const N: usize>() -> [WidgetId; N] {
    let mut ids = [WidgetId::reserved(0); N];

    for id in &mut ids {
        *id = WidgetId::next()
    }

    ids
}
