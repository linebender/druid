use crate::{WidgetPod, Env, Widget, Data, LifeCycle, EventCtx, PaintCtx, BoxConstraints, LifeCycleCtx, LayoutCtx, Event, UpdateCtx};
use druid::{Size, Point};

///
///
///
pub struct EnableIf<T, W> {
    inner: WidgetPod<T, W>,
    enable_if: Box<dyn Fn(&T, &Env) -> bool>
}

impl<T, W: Widget<T>> EnableIf<T, W> {
    /// creates a new EnableIf which disables the widget, if the closure returns `false`
    pub fn new(widget: W, enable_if: impl Fn(&T, &Env) -> bool + 'static) -> Self {
        EnableIf {
            inner: WidgetPod::new(widget),
            enable_if: Box::new(enable_if),
        }
    }

    /// creates a new EnableIf which disables the widget, if the closure returns `false`
    pub fn boxed(widget: W, enable_if: Box<dyn Fn(&T, &Env) -> bool>) -> Self {
        EnableIf {
            inner: WidgetPod::new(widget),
            enable_if,
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for EnableIf<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        let enabled = (self.enable_if)(data, env);
        ctx.set_enabled(enabled);

        self.inner.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_origin(ctx, data, env, Point::ZERO);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
    }
}