use crate::widget::prelude::*;
use std::any::{Any, TypeId};

/// A widget augmented with additional data (often used by its parent).
pub struct Augmented<W, Aug> {
    widget: W,
    aug: Aug,
}

impl<W, Aug> Augmented<W, Aug> {
    /// Create an Augmented widget
    pub fn new(widget: W, aug: Aug) -> Self {
        Augmented { widget, aug }
    }
}

impl<T, W: Widget<T>, Aug: 'static> Widget<T> for Augmented<W, Aug> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.widget.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.widget.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.widget.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        self.widget.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.widget.paint(ctx, data, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.widget.id()
    }

    fn augmentation_raw(&self, type_id: TypeId) -> Option<&dyn Any> {
        if TypeId::of::<Aug>() == type_id {
            Some(&self.aug)
        } else {
            self.widget.augmentation_raw(type_id)
        }
    }
}
