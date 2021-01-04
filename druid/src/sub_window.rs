use crate::app::{PendingWindow, WindowConfig};
use crate::command::sys::SUB_WINDOW_HOST_TO_PARENT;
use crate::commands::SUB_WINDOW_PARENT_TO_HOST;
use crate::lens::Unit;
use crate::widget::prelude::*;
use crate::win_handler::AppState;
use crate::{
    Command, Data, Point, Rect, Widget, WidgetExt, WidgetId, WidgetPod, WindowHandle, WindowId,
};
use druid_shell::Error;
use std::ops::Deref;

// We can't have any type arguments here, as both ends would need to know them
// ahead of time in order to instantiate correctly.
// So we erase everything to ()
/// The required information to create a sub window, including the widget it should host, and the
pub(crate) struct SubWindowRequirement {
    pub(crate) host_id: WidgetId,
    pub(crate) sub_window_root: Box<dyn Widget<()>>,
    pub(crate) window_config: WindowConfig,
    /// The window id that the sub window will have once it is created. Can be used to send commands to.
    pub window_id: WindowId,
}

impl SubWindowRequirement {
    /// Creates a subwindow requirement that hosts the provided widget within a sub window host.
    /// It will synchronise data updates with the provided parent_id if "sync" is true, and it will expect to be sent
    /// SUB_WINDOW_PARENT_TO_HOST commands to update the provided data for the widget.
    pub fn new<U, W: Widget<U>>(
        parent_id: WidgetId,
        window_config: WindowConfig,
        widget: W,
        data: U,
    ) -> SubWindowRequirement
    where
        W: 'static,
        U: Data,
    {
        let host_id = WidgetId::next();
        let sub_window_host = SubWindowHost::new(host_id, parent_id, data, widget).boxed();
        SubWindowRequirement {
            host_id,
            sub_window_root: sub_window_host,
            window_config,
            window_id: WindowId::next(),
        }
    }

    pub(crate) fn make_sub_window<T: Data>(
        self,
        app_state: &mut AppState<T>,
    ) -> Result<WindowHandle, Error> {
        let sub_window_root = self.sub_window_root;
        let pending = PendingWindow::new(|| sub_window_root.lens(Unit::default()));
        app_state.build_native_window(self.window_id, pending, self.window_config)
    }
}

struct SubWindowHost<U, W: Widget<U>> {
    id: WidgetId,
    parent_id: WidgetId,
    data: U,
    child: WidgetPod<U, W>,
}

impl<U, W: Widget<U>> SubWindowHost<U, W> {
    pub(crate) fn new(id: WidgetId, parent_id: WidgetId, data: U, widget: W) -> Self {
        SubWindowHost {
            id,
            parent_id,
            data,
            child: WidgetPod::new(widget),
        }
    }
}

impl<U: Data, W: Widget<U>> Widget<()> for SubWindowHost<U, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut (), env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SUB_WINDOW_PARENT_TO_HOST) => {
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
                if !old.same(&self.data) {
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
