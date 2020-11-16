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

//! Helper types for test writing.
//!
//! This includes tools for making throwaway widgets more easily.

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
    inner: WidgetPod<T, Box<dyn Widget<T>>>,
    replacer: Box<dyn Fn() -> Box<dyn Widget<T>>>,
}

/// A widget that records each time one of its methods is called.
///
/// Make one like this:
///
/// ```
/// let recording = Recording::default();
/// let widget = Label::new().padding(4.0).record(&recording);
///
/// Harness::create((), widget, |harness| {
///     widget.send_initial_events();
///     assert_matches!(recording.next(), Record::L(LifeCycle::WidgetAdded));
/// })
/// ```
pub struct Recorder<W> {
    recording: Recording,
    inner: W,
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
            inner: self,
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
        inner: impl Widget<T> + 'static,
        f: impl Fn() -> W + 'static,
    ) -> Self {
        let inner = WidgetPod::new(inner.boxed());
        let replacer = Box::new(move || f().boxed());
        ReplaceChild { inner, replacer }
    }
}

impl<T: Data> Widget<T> for ReplaceChild<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Command(cmd) = event {
            if cmd.is(REPLACE_CHILD) {
                self.inner = WidgetPod::new((self.replacer)());
                ctx.children_changed();
                return;
            }
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint_raw(ctx, data, env)
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
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let should_record = match event {
            LifeCycle::Internal(InternalLifeCycle::DebugRequestState { .. }) => false,
            LifeCycle::Internal(InternalLifeCycle::DebugInspectState(_)) => false,
            _ => true,
        };

        if should_record {
            self.recording.push(Record::L(event.clone()));
        }

        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
        self.recording
            .push(Record::Update(ctx.widget_state.invalid.clone()));
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.recording.push(Record::Layout(size));
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
        self.recording.push(Record::Paint)
    }
}

// easily make a bunch of WidgetIds
pub fn widget_id2() -> (WidgetId, WidgetId) {
    (WidgetId::next(), WidgetId::next())
}

pub fn widget_id3() -> (WidgetId, WidgetId, WidgetId) {
    (WidgetId::next(), WidgetId::next(), WidgetId::next())
}

pub fn widget_id4() -> (WidgetId, WidgetId, WidgetId, WidgetId) {
    (
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
    )
}

#[allow(dead_code)]
pub fn widget_id5() -> (WidgetId, WidgetId, WidgetId, WidgetId, WidgetId) {
    (
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
    )
}

pub fn widget_id6() -> (WidgetId, WidgetId, WidgetId, WidgetId, WidgetId, WidgetId) {
    (
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
        WidgetId::next(),
    )
}
