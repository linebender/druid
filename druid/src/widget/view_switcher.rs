use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, Size, UpdateCtx, Widget, WidgetPod,
};

pub struct ViewSwitcher<T, U>
where
    T: Data + PartialEq,
    U: Data,
{
    closure: Box<dyn Fn(&(T, U), &Env) -> Box<dyn Widget<(T, U)>>>,
    current_child: Option<WidgetPod<(T, U), Box<dyn Widget<(T, U)>>>>,
}

impl<T, U> ViewSwitcher<T, U>
where
    T: Data + PartialEq,
    U: Data,
{
    pub fn new(closure: impl Fn(&(T, U), &Env) -> Box<dyn Widget<(T, U)>> + 'static) -> Self {
        Self {
            closure: Box::new(closure),
            current_child: None,
        }
    }
}

impl<T, U> Widget<(T, U)> for ViewSwitcher<T, U>
where
    T: Data + PartialEq,
    U: Data,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut (T, U), env: &Env) {
        if let Some(ref mut child) = self.current_child {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &(T, U), env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.current_child = Some(WidgetPod::new((self.closure)(data, env)));
        }
        if let Some(ref mut child) = self.current_child {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &(T, U), data: &(T, U), env: &Env) {
        if self.current_child.is_none() || data.0 != old_data.0 {
            self.current_child = Some(WidgetPod::new((self.closure)(data, env)));
        }

        if let Some(ref mut child) = self.current_child {
            child.update(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &(T, U),
        env: &Env,
    ) -> Size {
        match self.current_child {
            Some(ref mut child) => {
                let size = child.layout(layout_ctx, bc, data, env);
                child.set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));
                size
            }
            None => bc.max(),
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &(T, U), env: &Env) {
        if let Some(ref mut child) = self.current_child {
            child.paint(ctx, data, env);
        }
    }
}
