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

//! A widget for optional data, with different `Some` and `None` children.

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget, WidgetExt, WidgetPod,
};

use druid::widget::SizedBox;

/// A widget that switches between two possible child views, for `Data` that
/// is `Option<T>`.
pub struct Maybe<T> {
    some_maker: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    none_maker: Box<dyn Fn() -> Box<dyn Widget<()>>>,
    widget: MaybeWidget<T>,
}

/// Internal widget, which is either the `Some` variant, or the `None` variant.
#[allow(clippy::large_enum_variant)]
enum MaybeWidget<T> {
    Some(WidgetPod<T, Box<dyn Widget<T>>>),
    None(WidgetPod<(), Box<dyn Widget<()>>>),
}

impl<T: Data> Maybe<T> {
    /// Create a new `Maybe` widget with a `Some` and a `None` branch.
    pub fn new<W1, W2>(
        // we make these generic so that the caller doesn't have to explicitly
        // box. We don't technically *need* to box, but it seems simpler.
        some_maker: impl Fn() -> W1 + 'static,
        none_maker: impl Fn() -> W2 + 'static,
    ) -> Maybe<T>
    where
        W1: Widget<T> + 'static,
        W2: Widget<()> + 'static,
    {
        let widget = MaybeWidget::Some(WidgetPod::new(some_maker().boxed()));
        Maybe {
            some_maker: Box::new(move || some_maker().boxed()),
            none_maker: Box::new(move || none_maker().boxed()),
            widget,
        }
    }

    /// Create a new `Maybe` widget where the `None` branch is an empty widget.
    pub fn or_empty<W1: Widget<T> + 'static>(some_maker: impl Fn() -> W1 + 'static) -> Maybe<T> {
        Self::new(some_maker, SizedBox::empty)
    }

    /// Re-create the internal widget, usually in response to the optional going `Some` -> `None`
    /// or the reverse.
    fn rebuild_widget(&mut self, is_some: bool) {
        if is_some {
            self.widget = MaybeWidget::Some(WidgetPod::new((self.some_maker)()));
        } else {
            self.widget = MaybeWidget::None(WidgetPod::new((self.none_maker)()));
        }
    }
}

impl<T: Data> Widget<Option<T>> for Maybe<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        if data.is_some() == self.widget.is_some() {
            match data.as_mut() {
                Some(d) => self.widget.with_some(|w| w.event(ctx, event, d, env)),
                None => self.widget.with_none(|w| w.event(ctx, event, &mut (), env)),
            };
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Option<T>,
        env: &Env,
    ) {
        if data.is_some() != self.widget.is_some() {
            // possible if getting lifecycle after an event that changed the data,
            // or on WidgetAdded
            self.rebuild_widget(data.is_some());
        }
        assert_eq!(data.is_some(), self.widget.is_some(), "{:?}", event);
        match data.as_ref() {
            Some(d) => self.widget.with_some(|w| w.lifecycle(ctx, event, d, env)),
            None => self.widget.with_none(|w| w.lifecycle(ctx, event, &(), env)),
        };
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Option<T>, data: &Option<T>, env: &Env) {
        if old_data.is_some() != data.is_some() {
            self.rebuild_widget(data.is_some());
            ctx.children_changed();
        } else {
            match data {
                Some(new) => self.widget.with_some(|w| w.update(ctx, new, env)),
                None => self.widget.with_none(|w| w.update(ctx, &(), env)),
            };
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Option<T>,
        env: &Env,
    ) -> Size {
        match data.as_ref() {
            Some(d) => self.widget.with_some(|w| {
                let size = w.layout(ctx, bc, d, env);
                w.set_layout_rect(ctx, d, env, size.to_rect());
                size
            }),
            None => self.widget.with_none(|w| {
                let size = w.layout(ctx, bc, &(), env);
                w.set_layout_rect(ctx, &(), env, size.to_rect());
                size
            }),
        }
        .unwrap_or_default()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Option<T>, env: &Env) {
        match data.as_ref() {
            Some(d) => self.widget.with_some(|w| w.paint(ctx, d, env)),
            None => self.widget.with_none(|w| w.paint(ctx, &(), env)),
        };
    }
}

impl<T> MaybeWidget<T> {
    /// Like `Option::is_some`.
    fn is_some(&self) -> bool {
        match self {
            Self::Some(_) => true,
            Self::None(_) => false,
        }
    }

    /// Lens to the `Some` variant.
    fn with_some<R, F: FnOnce(&mut WidgetPod<T, Box<dyn Widget<T>>>) -> R>(
        &mut self,
        f: F,
    ) -> Option<R> {
        match self {
            Self::Some(widget) => Some(f(widget)),
            Self::None(_) => {
                tracing::trace!("`MaybeWidget::with_some` called on `None` value");
                None
            }
        }
    }

    /// Lens to the `None` variant.
    fn with_none<R, F: FnOnce(&mut WidgetPod<(), Box<dyn Widget<()>>>) -> R>(
        &mut self,
        f: F,
    ) -> Option<R> {
        match self {
            Self::None(widget) => Some(f(widget)),
            Self::Some(_) => {
                tracing::trace!("`MaybeWidget::with_none` called on `Some` value");
                None
            }
        }
    }
}
