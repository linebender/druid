// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget that accepts a closure to update the environment for its child.

use crate::debug_state::DebugState;
use crate::widget::prelude::*;
use crate::widget::WidgetWrapper;
use crate::{Data, Point, WidgetPod};
use tracing::instrument;

/// A widget that accepts a closure to update the environment for its child.
pub struct EnvScope<T, W> {
    pub(crate) f: Box<dyn Fn(&mut Env, &T)>,
    pub(crate) child: WidgetPod<T, W>,
}

impl<T, W: Widget<T>> EnvScope<T, W> {
    /// Create a widget that updates the environment for its descendants.
    ///
    /// Accepts a closure that sets Env values.
    ///
    /// This is available as [`WidgetExt::env_scope`] for convenience.
    ///
    /// # Examples
    /// ```
    /// # use druid::{theme, Widget};
    /// # use druid::piet::{Color};
    /// # use druid::widget::{Label, EnvScope};
    /// # fn build_widget() -> impl Widget<String> {
    /// EnvScope::new(
    ///     |env, data| {
    ///         env.set(theme::TEXT_COLOR, Color::WHITE);
    ///     },
    ///     Label::new("White text!")
    /// )
    ///
    /// # }
    /// ```
    ///
    /// [`WidgetExt::env_scope`]: super::WidgetExt::env_scope
    pub fn new(f: impl Fn(&mut Env, &T) + 'static, child: W) -> EnvScope<T, W> {
        EnvScope {
            f: Box::new(f),
            child: WidgetPod::new(child),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for EnvScope<T, W> {
    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, data);

        self.child.event(ctx, event, data, &new_env)
    }

    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, data);
        self.child.lifecycle(ctx, event, data, &new_env)
    }

    #[instrument(
        name = "EnvScope",
        level = "trace",
        skip(self, ctx, _old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, data);

        self.child.update(ctx, data, &new_env);
    }

    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("EnvScope");

        let mut new_env = env.clone();
        (self.f)(&mut new_env, data);

        let size = self.child.layout(ctx, bc, data, &new_env);
        self.child.set_origin(ctx, Point::ORIGIN);
        size
    }

    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let mut new_env = env.clone();
        (self.f)(&mut new_env, data);

        self.child.paint(ctx, data, &new_env);
    }

    fn debug_state(&self, data: &T) -> DebugState {
        DebugState {
            display_name: self.short_type_name().to_string(),
            children: vec![self.child.widget().debug_state(data)],
            ..Default::default()
        }
    }
}

impl<T, W: Widget<T>> WidgetWrapper for EnvScope<T, W> {
    widget_wrapper_pod_body!(W, child);
}
