use std::fmt::Display;
use std::mem;
use std::str::FromStr;

use crate::kurbo::Size;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    UpdateCtx, Widget, WidgetId,
};

/// Converts a `Widget<String>` to a `Widget<Option<T>>`, mapping parse errors to None
pub struct Parse<T> {
    widget: T,
    state: String,
}

impl<T> Parse<T> {
    pub fn new(widget: T) -> Self {
        Self {
            widget,
            state: String::new(),
        }
    }
}

impl<T: FromStr + Display + Data, W: Widget<String>> Widget<Option<T>> for Parse<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        self.widget.event(ctx, event, &mut self.state, env);
        *data = self.state.parse().ok();
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Option<T>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if let Some(data) = data {
                self.state = data.to_string();
            }
        }
        self.widget.lifecycle(ctx, event, &self.state, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &Option<T>, data: &Option<T>, env: &Env) {
        let old = match *data {
            None => return, // Don't clobber the input
            Some(ref x) => mem::replace(&mut self.state, x.to_string()),
        };
        self.widget.update(ctx, &old, &self.state, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &Option<T>,
        env: &Env,
    ) -> Size {
        self.widget.layout(ctx, bc, &self.state, env)
    }

    fn paint(&mut self, paint: &mut PaintCtx, _data: &Option<T>, env: &Env) {
        self.widget.paint(paint, &self.state, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.widget.id()
    }
}
