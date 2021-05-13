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

//! A widget that accepts a closure to update the environment for its child.

use crate::widget::prelude::*;
use crate::widget::WidgetWrapper;
use crate::{Data, Point, WidgetPod};
use tracing::instrument;

/// A widget that accepts a closure to update the environment for its child.
pub struct EnvScope<T, W> {
    pub(crate) child: WidgetPod<T, W>,
    pub(crate) current_child_env: Option<Env>,
    pub(crate) prev_super_env: Option<Env>,
    pub(crate) overrides: EnvOverride<T>,
    // pub(crate) modify_env: Option<Box<dyn Fn(&T, &mut Env)>>,
    // pub(crate) should_modify_env_now: Option<EnvInvalidationCheck<T>>,
}

pub(crate) enum EnvOverride<T> {
    Static(Env),
    Dynamic(DynamicEnv<T>, EnvInvalidationCheck<T>),
}
type DynamicEnv<T> = Box<dyn Fn(&T, &mut Env)>;
type EnvInvalidationCheck<T> = Box<dyn Fn(&T, &T, &Env) -> bool>;

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
    /// [`WidgetExt::env_scope`]: crate::WidgetExt::env_scope
    pub fn new(overrides: Env, child: W) -> EnvScope<T, W> {
        EnvScope {
            child: WidgetPod::new(child),
            current_child_env: None,
            prev_super_env: None,
            overrides: EnvOverride::Static(overrides),
        }
    }

    /// Create a widget that updates the environment for its descendants based on
    /// application data.
    ///
    /// This accepts two closures. The `invalidate_env` argument will determine whether
    /// the `overrides` is called to update `Env`.
    pub fn dynamic(
        overrides: impl Fn(&T, &mut Env) + 'static,
        invalidate_env: impl Fn(&T, &T, &Env) -> bool + 'static,
        child: W,
    ) -> EnvScope<T, W> {
        EnvScope {
            child: WidgetPod::new(child),
            current_child_env: None,
            prev_super_env: None,
            overrides: EnvOverride::Dynamic(Box::new(overrides), Box::new(invalidate_env)),
        }
    }

    fn child_env(&mut self, super_env: &Env, data: &T) {
        let super_same = self
            .prev_super_env
            .as_ref()
            .map(|old| old.same(&super_env))
            .unwrap_or(false);

        if !super_same {
            match self.overrides {
                EnvOverride::Static(ref overrides) => {
                    self.current_child_env = Some(super_env.with_overrides(overrides));
                }
                EnvOverride::Dynamic(ref apply_overrides, _) => {
                    let mut new_env = super_env.clone();
                    (apply_overrides)(data, &mut new_env);
                    self.current_child_env = Some(new_env);
                }
            }
            self.prev_super_env = Some(super_env.clone());
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for EnvScope<T, W> {
    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child_env(env, &data);
        let child_env = self.current_child_env.as_ref().unwrap_or(env);

        self.child.event(ctx, event, data, child_env)
    }

    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            if let EnvOverride::Dynamic(ref apply_overrides, _) = self.overrides {
                let mut new_env = env.clone();
                (apply_overrides)(data, &mut new_env);
                self.current_child_env = Some(new_env);
                self.prev_super_env = Some(env.clone());
            }
        }
        self.child_env(env, data);
        let child_env = self.current_child_env.as_ref().unwrap_or(env);

        self.child.lifecycle(ctx, event, data, &child_env)
    }

    #[instrument(
        name = "EnvScope",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let EnvOverride::Dynamic(ref apply_overrides, ref invalidate) = self.overrides {
            let should_invalidate_env = (invalidate)(old_data, data, env);

            if should_invalidate_env {
                let mut new_env = env.clone();
                (apply_overrides)(data, &mut new_env);
                self.current_child_env = Some(new_env);
                self.prev_super_env = Some(env.clone());
            }
        }

        if ctx.env_changed() {
            self.child_env(env, data);
        }

        let child_env = self.current_child_env.as_ref().unwrap_or(&env);
        self.child.update(ctx, data, &child_env);
    }

    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("EnvScope");
        self.child_env(env, data);
        let child_env = self.current_child_env.as_ref().unwrap_or(env);

        let size = self.child.layout(ctx, &bc, data, &child_env);
        self.child.set_origin(ctx, data, &child_env, Point::ORIGIN);
        size
    }

    #[instrument(name = "EnvScope", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child_env(env, data);
        let child_env = self.current_child_env.as_ref().unwrap_or(env);

        self.child.paint(ctx, data, &child_env);
    }
}

impl<T, W: Widget<T>> WidgetWrapper for EnvScope<T, W> {
    widget_wrapper_pod_body!(W, child);
}
