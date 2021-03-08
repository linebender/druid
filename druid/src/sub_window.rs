// Copyright 2021 The Druid Authors.
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

use crate::app::{PendingWindow, WindowConfig};
use crate::commands::{SUB_WINDOW_HOST_TO_PARENT, SUB_WINDOW_PARENT_TO_HOST};
use crate::lens::Unit;
use crate::widget::prelude::*;
use crate::win_handler::AppState;
use crate::{
    Command, Data, Point, Rect, Widget, WidgetExt, WidgetId, WidgetPod, WindowHandle, WindowId,
};
use druid_shell::Error;
use std::any::Any;
use std::ops::Deref;
use tracing::{instrument, warn};
// We can't have any type arguments here, as both ends would need to know them
// ahead of time in order to instantiate correctly.
// So we erase everything to ()
/// The required information to create a sub window, including the widget it should host, and the
/// config of the window to be created.
pub(crate) struct SubWindowDesc {
    pub(crate) host_id: WidgetId,
    pub(crate) sub_window_root: Box<dyn Widget<()>>,
    pub(crate) window_config: WindowConfig,
    /// The window id that the sub window will have once it is created. Can be used to send commands to.
    pub window_id: WindowId,
}

pub(crate) struct SubWindowUpdate {
    pub(crate) data: Option<Box<dyn Any>>,
    pub(crate) env: Option<Env>,
}

impl SubWindowDesc {
    /// Creates a subwindow requirement that hosts the provided widget within a sub window host.
    /// It will synchronise data updates with the provided parent_id if "sync" is true, and it will expect to be sent
    /// SUB_WINDOW_PARENT_TO_HOST commands to update the provided data for the widget.
    pub fn new<U, W: Widget<U>>(
        parent_id: WidgetId,
        window_config: WindowConfig,
        widget: W,
        data: U,
        env: Env,
    ) -> SubWindowDesc
    where
        W: 'static,
        U: Data,
    {
        let host_id = WidgetId::next();
        let sub_window_host = SubWindowHost::new(host_id, parent_id, widget, data, env).boxed();
        SubWindowDesc {
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
        let pending = PendingWindow::new(sub_window_root.lens(Unit::default()));
        app_state.build_native_window(self.window_id, pending, self.window_config)
    }
}

struct SubWindowHost<U, W: Widget<U>> {
    id: WidgetId,
    parent_id: WidgetId,
    child: WidgetPod<U, W>,
    data: U,
    env: Env,
}

impl<U, W: Widget<U>> SubWindowHost<U, W> {
    pub(crate) fn new(id: WidgetId, parent_id: WidgetId, widget: W, data: U, env: Env) -> Self {
        SubWindowHost {
            id,
            parent_id,
            data,
            env,
            child: WidgetPod::new(widget),
        }
    }
}

impl<U: Data, W: Widget<U>> Widget<()> for SubWindowHost<U, W> {
    #[instrument(
        name = "SubWindowHost",
        level = "trace",
        skip(self, ctx, event, _data, _env)
    )]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut (), _env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SUB_WINDOW_PARENT_TO_HOST) => {
                let update = cmd.get_unchecked(SUB_WINDOW_PARENT_TO_HOST);
                if let Some(data_update) = &update.data {
                    if let Some(dc) = data_update.downcast_ref::<U>() {
                        self.data = dc.deref().clone();
                        ctx.request_update();
                    } else {
                        warn!("Received a sub window parent to host command that could not be unwrapped. \
                        This could mean that the sub window you requested and the enclosing widget pod that you opened it from do not share a common data type. \
                        Make sure you have a widget pod between your requesting widget and any lenses." )
                    }
                }
                if let Some(env_update) = &update.env {
                    self.env = env_update.clone()
                }
                ctx.set_handled();
            }
            _ => {
                let old = self.data.clone(); // Could avoid this by keeping two bit of data or if we could ask widget pod?
                self.child.event(ctx, event, &mut self.data, &self.env);
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

    #[instrument(
        name = "SubWindowHost",
        level = "trace",
        skip(self, ctx, event, _data, _env)
    )]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &(), _env: &Env) {
        self.child.lifecycle(ctx, event, &self.data, &self.env)
    }

    #[instrument(
        name = "SubWindowHost",
        level = "trace",
        skip(self, ctx, _old_data, _data, _env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &(), _data: &(), _env: &Env) {
        if ctx.has_requested_update() {
            self.child.update(ctx, &self.data, &self.env);
        }
    }

    #[instrument(
        name = "SubWindowHost",
        level = "trace",
        skip(self, ctx, bc, _data, _env)
    )]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &(), _env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, &self.data, &self.env);
        self.child.set_layout_rect(
            ctx,
            &self.data,
            &self.env,
            Rect::from_origin_size(Point::ORIGIN, size),
        );
        size
    }

    #[instrument(name = "SubWindowHost", level = "trace", skip(self, ctx, _data, _env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &(), _env: &Env) {
        self.child.paint_raw(ctx, &self.data, &self.env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(self.id)
    }
}
