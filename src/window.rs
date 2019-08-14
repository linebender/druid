// Copyright 2019 The xi-editor Authors.
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

//! Management of multiple windows.

use std::collections::HashMap;

use crate::kurbo::{Point, Rect, Size};

use druid_shell::window::{Cursor, Text, WinCtx, WindowHandle};

use crate::win_handler::WindowState;
use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
    WidgetPod,
};

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
// TODO: Remove Default when we get it fully wired up
#[derive(Default)]
pub struct WindowId(u32);

/// A container for a set of windows.
pub struct WindowSet<T: Data> {
    map: HashMap<WindowId, WindowPod<T>>,
}

// The state for a single window. Right now, access from user code is only through
// `WindowSet`, but this struct is split out in case more flexibility is needed.
struct WindowPod<T: Data> {
    root: WidgetPod<T, Box<dyn Widget<T>>>,
    size: Size,
}

/// A state and behavior object for the application logic holding the windows.
///
/// A main function of this object is to hold a `WindowSet`, but additional
/// state and logic may be added.
///
/// All event flow is passed through this object, which then delegates it down
/// the hierarchy.
// TODO: consider rename?
pub trait RootWidget<T> {
    /// Propagate an event to a child window.
    // Note: this signature is different from the widget trait in that it doesn't return an
    // `Action`. We still have to think about this.
    fn event(&mut self, event: &Event, ctx: &mut EventCtxRoot, data: &mut T, env: &Env);

    /// Propagate a data update to all windows.
    fn update(&mut self, ctx: &mut UpdateCtxRoot, data: &T, env: &Env);

    /// Propagate layout to a child window.
    ///
    /// The case for this method is weak; it could be subsumed into the `paint` method.
    fn layout(&mut self, ctx: &mut LayoutCtxRoot, data: &T, env: &Env);

    /// Paint a child window's appearance.
    fn paint(&mut self, paint_ctx: &mut PaintCtxRoot, data: &T, env: &Env);
}

pub struct EventCtxRoot<'a, 'b> {
    pub(crate) win_ctx: &'a mut dyn WinCtx<'b>,
    pub(crate) cursor: &'a mut Option<Cursor>,
    // TODO: migrate most usage of `WindowHandle` to `WinCtx` instead.
    pub(crate) window: &'a WindowHandle,
    pub(crate) base_state: BaseState,
    pub(crate) is_handled: bool,
    pub(crate) window_id: WindowId,
    // TODO: mutable access to the handle map, so we can add windows
}

pub struct UpdateCtxRoot<'a, 'b: 'a> {
    pub(crate) window_state: &'a HashMap<WindowId, WindowState>,
    pub(crate) text_factory: &'a mut Text<'b>,
    pub(crate) originating_window: WindowId,
    pub(crate) needs_inval: bool,
}

pub struct LayoutCtxRoot<'a, 'b>(pub(crate) LayoutCtx<'a, 'b>);

pub struct PaintCtxRoot<'a, 'b>(pub(crate) PaintCtx<'a, 'b>);

impl<T: Data> WindowSet<T> {
    pub fn event(&mut self, event: &Event, event_ctx: &mut EventCtxRoot, data: &mut T, env: &Env) {
        if let Some(root) = self.map.get_mut(&event_ctx.window_id) {
            root.event(event, event_ctx, data, env);
        }
    }

    pub fn update(&mut self, root_ctx: &mut UpdateCtxRoot, data: &mut T, env: &Env) {
        for (window_id, root) in &mut self.map {
            let mut update_ctx = UpdateCtx {
                text_factory: root_ctx.text_factory,
                window: &root_ctx.window_state.get(window_id).unwrap().handle,
                needs_inval: false,
                window_id: *window_id,
            };
            root.update(&mut update_ctx, data, env);
            if *window_id == root_ctx.originating_window {
                root_ctx.needs_inval = update_ctx.needs_inval;
            } else {
                update_ctx.window.invalidate();
            }
        }
    }

    pub fn layout(&mut self, layout_ctx: &mut LayoutCtxRoot, data: &T, env: &Env) {
        if let Some(root) = self.map.get_mut(&layout_ctx.0.window_id) {
            root.layout(layout_ctx, data, env);
        }
    }

    pub fn paint(&mut self, paint_ctx: &mut PaintCtxRoot, data: &T, env: &Env) {
        if let Some(root) = self.map.get_mut(&paint_ctx.0.window_id) {
            root.paint(paint_ctx, data, env);
        }
    }
}

impl<T: Data> WindowPod<T> {
    pub fn event(&mut self, event: &Event, root_ctx: &mut EventCtxRoot, data: &mut T, env: &Env) {
        let mut base_state = Default::default();
        let mut ctx = EventCtx {
            win_ctx: root_ctx.win_ctx,
            cursor: root_ctx.cursor,
            window: root_ctx.window,
            base_state: &mut base_state,
            had_active: self.root.state.has_active,
            is_handled: false,
        };
        let _action = self.root.event(event, &mut ctx, data, env);
        root_ctx.is_handled = ctx.is_handled;

        if let Some(cursor) = root_ctx.cursor {
            root_ctx.win_ctx.set_cursor(&cursor);
        }
    }

    pub fn update(&mut self, update_ctx: &mut UpdateCtx, data: &mut T, env: &Env) {
        self.root.update(update_ctx, data, env);
    }

    pub fn layout(&mut self, layout_ctx: &mut LayoutCtxRoot, data: &T, env: &Env) {
        let bc = BoxConstraints::tight(self.size);
        let size = self.root.layout(&mut layout_ctx.0, &bc, data, env);
        self.root
            .set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));
    }

    pub fn paint(&mut self, paint_ctx: &mut PaintCtxRoot, data: &T, env: &Env) {
        self.root.paint(&mut paint_ctx.0, data, env);
    }
}
