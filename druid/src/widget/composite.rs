use std::marker::PhantomData;

use crate::kurbo::{Point, Rect, Size};
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetPod,
};

/// Meta information for Widget derive
pub struct CompositeMeta<T> {
    /// Built widget
    pub widget: Option<WidgetPod<T, Box<dyn Widget<T>>>>,
}

impl<T> Default for CompositeMeta<T> {
    fn default() -> Self {
        CompositeMeta { widget: None }
    }
}

/// Composable widget
pub struct CompositeWidget<T, W: Widget<T> + 'static, B: CompositeBuild<T, W>> {
    child: Option<WidgetPod<T, Box<dyn Widget<T>>>>,
    composite_build: B,
    phantom: PhantomData<W>,
}

impl<T, W: Widget<T> + 'static, B: CompositeBuild<T, W>> CompositeWidget<T, W, B> {
    /// Create composite widget
    pub fn new(composite_build: B) -> Self {
        CompositeWidget {
            child: None,
            composite_build,
            phantom: PhantomData::default(),
        }
    }
}

/// Trait that build widget
pub trait CompositeBuild<T, W: Widget<T>> {
    /// Build composite widget
    fn build(&self) -> W;
}

impl<T: Data, W: Widget<T> + 'static, B: CompositeBuild<T, W>> Widget<T>
    for CompositeWidget<T, W, B>
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(child) = &mut self.child {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.child
                .replace(WidgetPod::new(self.composite_build.build()).boxed());
        }

        if let Some(child) = &mut self.child {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        if let Some(child) = &mut self.child {
            child.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        match &mut self.child {
            Some(child) => {
                let size = child.layout(ctx, &bc, data, env);
                let rect = Rect::from_origin_size(Point::ORIGIN, size);
                child.set_layout_rect(ctx, data, env, rect);
                size
            }
            None => Size::ZERO,
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(child) = &mut self.child {
            child.paint(ctx, data, env);
        }
    }
}
