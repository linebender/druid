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

//! A widget that constrains its child with the provided [BoxConstraints].
//!
//! [BoxConstraints]: druid::box_constraints::BoxConstraints

use tracing::{instrument, trace, warn};

use crate::widget::prelude::*;
use crate::Data;

/// A widget that constrains its child with the provided [BoxConstraints].
///
/// If given a child, this widget forces its child to have a width and height
/// between the provided min and max box constraints.
///
/// If not given a child, `ConstrainedBox` will try to size itself to its own provided
/// max dimensions within the parent's constraints.
///
/// [BoxConstraints]: druid::box_constraints::BoxConstraints
pub struct ConstrainedBox<T> {
    inner: Option<Box<dyn Widget<T>>>,
    bc: BoxConstraints,
}

impl<T> ConstrainedBox<T> {
    /// Construct container with child and provided box constraints.
    pub fn new(inner: impl Widget<T> + 'static, bc: impl Into<BoxConstraints>) -> Self {
        Self {
            inner: Some(Box::new(inner)),
            bc: bc.into(),
        }
    }

    /// Construct container without child, but with box constraints.
    pub fn empty(bc: impl Into<BoxConstraints>) -> Self {
        Self {
            inner: None,
            bc: bc.into(),
        }
    }

    /// Get the current box constraints.
    pub fn constraints(&self) -> BoxConstraints {
        self.bc
    }

    /// Changes current box constraints to the provided constraints.
    pub fn set_constraints(&mut self, bc: BoxConstraints) {
        self.bc = bc;
    }

    /// Makes the box constraints minimum size 0.0 for both width and height
    /// and keeps the same maximum size.
    pub fn loosen(&mut self) {
        self.bc = self.bc.loosen();
    }
}

impl<T: Data> Widget<T> for ConstrainedBox<T> {
    #[instrument(
        name = "ConstrainedBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.event(ctx, event, data, env);
        }
    }

    #[instrument(
        name = "ConstrainedBox",
        level = "trace",
        skip(self, ctx, event, data, env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.lifecycle(ctx, event, data, env)
        }
    }

    #[instrument(
        name = "ConstrainedBox",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.update(ctx, old_data, data, env);
        }
    }

    #[instrument(
        name = "ConstrainedBox",
        level = "trace",
        skip(self, ctx, bc, data, env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("ConstrainedBox");

        let max = self.bc.max().clamp(bc.min(), bc.max());
        let min = self.bc.min().clamp(bc.min(), bc.max());
        let child_bc = BoxConstraints::new(min, max);

        let size = if let Some(inner) = self.inner.as_mut() {
            inner.layout(ctx, &child_bc, data, env)
        } else {
            child_bc.max()
        };

        trace!("Computed size: {}", size);
        if size.width.is_infinite() {
            warn!("ConstrainedBox is returning an infinite width.");
        }

        if size.height.is_infinite() {
            warn!("ConstrainedBox is returning an infinite height.");
        }

        size
    }

    #[instrument(name = "ConstrainedBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.paint(ctx, data, env);
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.as_ref().and_then(|inner| inner.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::harness::*;
    use crate::widget::Label;
    use crate::WidgetExt;

    #[test]
    fn tight_parent_constraints() {
        let id = WidgetId::next();

        let text = Label::new("hello!");

        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
        let constrained = ConstrainedBox::<()>::new(text, bc).with_id(id);
        let constrained = constrained.fix_width(400.).height(310.).center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(400., 310.));
        });
    }

    #[test]
    fn constrain_both_dimensions() {
        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));

        let id = WidgetId::next();
        let label = Label::new("hello!").fix_height(600.).width(800.);
        let constrained = ConstrainedBox::<()>::new(label, bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        // constrain to max
        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(250., 250.));
        });

        let label = Label::new("hello!").fix_width(75.).height(100.);
        let constrained = ConstrainedBox::<()>::new(label, bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        // constrain to min
        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(100., 125.));
        });
    }

    #[test]
    fn constrain_max_height() {
        let id = WidgetId::next();
        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
        let label = Label::new("hello!").fix_height(600.).width(200.);
        let constrained = ConstrainedBox::<()>::new(label, bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(200., 250.));
        });
    }

    #[test]
    fn constrain_max_width() {
        let id = WidgetId::next();
        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
        let label = Label::new("hello!").fix_width(600.).height(200.);
        let constrained = ConstrainedBox::<()>::new(label, bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(250., 200.));
        });
    }

    #[test]
    fn constrain_min_height() {
        let id = WidgetId::next();
        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
        let label = Label::new("hello!").fix_width(200.).height(50.);
        let constrained = ConstrainedBox::<()>::new(label, bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(200., 125.));
        });
    }

    #[test]
    fn constrain_min_width() {
        let id = WidgetId::next();
        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
        let label = Label::new("hello!").fix_width(25.).height(200.);
        let constrained = ConstrainedBox::<()>::new(label, bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(100., 200.));
        });
    }
    #[test]
    fn without_child() {
        let id = WidgetId::next();
        let bc = BoxConstraints::new(Size::new(100., 125.), Size::new(250., 250.));
        let constrained = ConstrainedBox::<()>::empty(bc).with_id(id).center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), constrained, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(250., 250.));
        });
    }
}
