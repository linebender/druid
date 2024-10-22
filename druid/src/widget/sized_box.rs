// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A widget with predefined size.

use crate::debug_state::DebugState;
use tracing::{instrument, trace, warn};

use crate::widget::prelude::*;
use crate::widget::Axis;
use crate::{Data, KeyOrValue};

/// A widget with predefined size.
///
/// If given a child, this widget forces its child to have a specific width and/or height
/// (assuming values are permitted by this widget's parent). If either the width or height is not
/// set, this widget will size itself to match the child's size in that dimension.
///
/// If not given a child, SizedBox will try to size itself as close to the specified height
/// and width as possible given the parent's constraints. If height or width is not set,
/// it will be treated as zero.
pub struct SizedBox<T> {
    child: Option<Box<dyn Widget<T>>>,
    width: Option<KeyOrValue<f64>>,
    height: Option<KeyOrValue<f64>>,
}

impl<T> SizedBox<T> {
    /// Construct container with child, and both width and height not set.
    pub fn new(child: impl Widget<T> + 'static) -> Self {
        Self {
            child: Some(Box::new(child)),
            width: None,
            height: None,
        }
    }

    /// Construct container without child, and both width and height not set.
    ///
    /// If the widget is unchanged, it will do nothing, which can be useful if you want to draw a
    /// widget some of the time (for example, it is used to implement
    /// [`Maybe`][crate::widget::Maybe]).
    #[doc(alias = "null")]
    pub fn empty() -> Self {
        Self {
            child: None,
            width: None,
            height: None,
        }
    }

    /// Set container's width.
    pub fn width(mut self, width: impl Into<KeyOrValue<f64>>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: impl Into<KeyOrValue<f64>>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(KeyOrValue::Concrete(f64::INFINITY));
        self.height = Some(KeyOrValue::Concrete(f64::INFINITY));
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(KeyOrValue::Concrete(f64::INFINITY));
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(KeyOrValue::Concrete(f64::INFINITY));
        self
    }

    fn child_constraints(&self, bc: &BoxConstraints, env: &Env) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let (min_width, max_width) = match &self.width {
            Some(width) => {
                let width = width.resolve(env);
                let w = width.clamp(bc.min().width, bc.max().width);
                (w, w)
            }
            None => (bc.min().width, bc.max().width),
        };

        let (min_height, max_height) = match &self.height {
            Some(height) => {
                let height = height.resolve(env);
                let h = height.clamp(bc.min().height, bc.max().height);
                (h, h)
            }
            None => (bc.min().height, bc.max().height),
        };

        BoxConstraints::new(
            Size::new(min_width, min_height),
            Size::new(max_width, max_height),
        )
    }

    #[cfg(test)]
    pub(crate) fn width_and_height(&self, env: &Env) -> (Option<f64>, Option<f64>) {
        (
            self.width.as_ref().map(|w| w.resolve(env)),
            self.height.as_ref().map(|h| h.resolve(env)),
        )
    }
}

impl<T: Data> Widget<T> for SizedBox<T> {
    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, event, data, env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(ref mut child) = self.child {
            child.event(ctx, event, data, env);
        }
    }

    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let Some(ref mut child) = self.child {
            child.lifecycle(ctx, event, data, env)
        }
    }

    #[instrument(
        name = "SizedBox",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(ref mut child) = self.child {
            child.update(ctx, old_data, data, env);
        }
    }

    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("SizedBox");

        let child_bc = self.child_constraints(bc, env);
        let size = match self.child.as_mut() {
            Some(child) => child.layout(ctx, &child_bc, data, env),
            None => bc.constrain((
                self.width
                    .as_ref()
                    .unwrap_or(&KeyOrValue::Concrete(0.0))
                    .resolve(env),
                self.height
                    .as_ref()
                    .unwrap_or(&KeyOrValue::Concrete(0.0))
                    .resolve(env),
            )),
        };

        trace!("Computed size: {}", size);
        if size.width.is_infinite() {
            warn!("SizedBox is returning an infinite width.");
        }

        if size.height.is_infinite() {
            warn!("SizedBox is returning an infinite height.");
        }

        size
    }

    #[instrument(name = "SizedBox", level = "trace", skip(self, ctx, data, env))]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut child) = self.child {
            child.paint(ctx, data, env);
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.child.as_ref().and_then(|child| child.id())
    }

    fn debug_state(&self, data: &T) -> DebugState {
        let children = if let Some(child) = &self.child {
            vec![child.debug_state(data)]
        } else {
            vec![]
        };
        DebugState {
            display_name: self.short_type_name().to_string(),
            children,
            ..Default::default()
        }
    }

    fn compute_max_intrinsic(
        &mut self,
        axis: Axis,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        let kv = match axis {
            Axis::Horizontal => self.width.as_ref(),
            Axis::Vertical => self.height.as_ref(),
        };
        match (self.child.as_mut(), kv) {
            (Some(c), Some(v)) => {
                let v = v.resolve(env);
                if v == f64::INFINITY {
                    c.compute_max_intrinsic(axis, ctx, bc, data, env)
                } else {
                    v
                }
            }
            (Some(c), None) => c.compute_max_intrinsic(axis, ctx, bc, data, env),
            (None, Some(v)) => {
                let v = v.resolve(env);
                if v == f64::INFINITY {
                    // If v infinite, we can only warn.
                    warn!("SizedBox is without a child and its dim is infinite. Either give SizedBox a child or make its dim finite. ")
                }
                v
            }
            (None, None) => 0.,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{widget::Label, Key};
    use test_log::test;

    #[test]
    fn expand() {
        let env = Env::empty();
        let expand = SizedBox::<()>::new(Label::new("hello!")).expand();
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc, &env);
        assert_eq!(child_bc.min(), Size::new(400., 400.,));
    }

    #[test]
    fn no_width() {
        let mut env = Env::empty();

        let expand = SizedBox::<()>::new(Label::new("hello!")).height(200.);
        let bc = BoxConstraints::tight(Size::new(400., 400.)).loosen();
        let child_bc = expand.child_constraints(&bc, &env);
        assert_eq!(child_bc.min(), Size::new(0., 200.,));
        assert_eq!(child_bc.max(), Size::new(400., 200.,));

        const HEIGHT_KEY: Key<f64> = Key::new("test-no-width-height");
        env.set(HEIGHT_KEY, 200.);
        let expand = SizedBox::<()>::new(Label::new("hello!")).height(HEIGHT_KEY);
        let child_bc = expand.child_constraints(&bc, &env);
        assert_eq!(child_bc.min(), Size::new(0., 200.,));
        assert_eq!(child_bc.max(), Size::new(400., 200.,));
    }
}
