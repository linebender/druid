use crate::{widget::prelude::*, Data};

pub struct Immediate<D, W: Widget<()>> {
    constructor: Box<dyn Fn(&D) -> W>,
    content: Option<W>,
}

impl<D, W: Widget<()>> Immediate<D, W> {
    pub fn new(constructor: impl Fn(&D) -> W + 'static) -> Self {
        Self {
            constructor: Box::new(constructor),
            content: None,
        }
    }
}

impl<D: Data, W: Widget<()>> Widget<D> for Immediate<D, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut D, env: &Env) {
        if let Some(content) = &mut self.content {
            content.event(ctx, event, &mut (), env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &D, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.content = Some((self.constructor)(data));
        }
        if let Some(content) = &mut self.content {
            content.lifecycle(ctx, event, &(), env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &D, data: &D, env: &Env) {
        if !old_data.same(data) {
            self.content = Some((self.constructor)(data));
            ctx.children_changed();
        } else {
            // This can happen when env changes, right?
            if let Some(content) = &mut self.content {
                content.update(ctx, &(), &(), env);
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &D, env: &Env) -> Size {
        if let Some(content) = &mut self.content {
            content.layout(ctx, bc, &(), env)
        } else {
            Size::ZERO
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &D, env: &Env) {
        if let Some(content) = &mut self.content {
            content.paint(ctx, &(), env);
        }
    }
}
