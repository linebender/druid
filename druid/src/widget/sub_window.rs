use crate::app::WindowConfig;
use crate::command::sys::SUB_WINDOW_PARENT_TO_HOST;
use crate::commands::SUB_WINDOW_HOST_TO_PARENT;
use crate::{
    BoxConstraints, Command, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Point, Rect, Size, SubWindowRequirement, UpdateCtx, Widget, WidgetExt, WidgetId,
    WidgetPod, WindowId,
};
use std::ops::Deref;

/// A widget currently meant to be used as the root of a sub window.
/// It ignores its own AppData, but provides its child with data synchronised
/// from the WidgetPod that created it.
pub struct SubWindowHost<U, W: Widget<U>> {
    id: WidgetId,
    parent_id: WidgetId,
    sync: bool,
    data: U,
    child: WidgetPod<U, W>,
}

impl<U, W: Widget<U>> SubWindowHost<U, W> {
    fn new(id: WidgetId, parent_id: WidgetId, sync: bool, data: U, widget: W) -> Self {
        SubWindowHost {
            id,
            parent_id,
            sync,
            data,
            child: WidgetPod::new(widget),
        }
    }

    /// Creates a subwindow requirement that hosts the provided widget within a sub window host.
    /// It will synchronise data updates with the provided parent_id if "sync" is true, and it will expect to be sent
    /// SUB_WINDOW_PARENT_TO_HOST commands to update the provided data for the widget.
    pub fn make_requirement(
        parent_id: WidgetId,
        window_config: WindowConfig,
        sync: bool,
        widget: W,
        data: U,
    ) -> SubWindowRequirement
    where
        W: 'static,
        U: Data,
    {
        let host_id = WidgetId::next();
        let sub_window_host = SubWindowHost::new(host_id, parent_id, sync, data, widget).boxed();
        let host_id = if sync { Some(host_id) } else { None };
        SubWindowRequirement::new(host_id, sub_window_host, window_config, WindowId::next())
    }
}

impl<U: Data, W: Widget<U>> Widget<()> for SubWindowHost<U, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut (), env: &Env) {
        match event {
            Event::Command(cmd) if self.sync && cmd.is(SUB_WINDOW_PARENT_TO_HOST) => {
                if let Some(update) = cmd
                    .get_unchecked(SUB_WINDOW_PARENT_TO_HOST)
                    .downcast_ref::<U>()
                {
                    self.data = update.deref().clone();
                    ctx.request_update();
                } else {
                    log::warn!("Received a sub window parent to host command that could not be unwrapped. \
                    This could mean that the sub window you requested and the enclosing widget pod that you opened it from do not share a common data type. \
                    Make sure you have a widget pod between your requesting widget and any lenses." )
                }
                ctx.set_handled();
            }
            _ => {
                let old = self.data.clone(); // Could avoid this by keeping two bit of data or if we could ask widget pod?
                self.child.event(ctx, event, &mut self.data, env);
                if self.sync && !old.same(&self.data) {
                    ctx.submit_command(Command::new(
                        SUB_WINDOW_HOST_TO_PARENT,
                        Box::new(self.data.clone()),
                        self.parent_id,
                    ))
                }
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &(), env: &Env) {
        self.child.lifecycle(ctx, event, &self.data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &(), _data: &(), env: &Env) {
        if ctx.has_requested_update() {
            // Should env be copied from the parent too? Possibly
            self.child.update(ctx, &self.data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &(), env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, &self.data, env);
        self.child.set_layout_rect(
            ctx,
            &self.data,
            env,
            Rect::from_origin_size(Point::ORIGIN, size),
        );
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &(), env: &Env) {
        self.child.paint_raw(ctx, &self.data, env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }
}
