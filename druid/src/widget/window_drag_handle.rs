// Copyright 2018 The Druid Authors.
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

//! A [`Controller`] widget that makes the target acts as a window grab handle.
//!
//! [`Controller`]: struct.Controller.html

use tracing::instrument;

use crate::widget::{Controller, ControllerHost};
use crate::widget::prelude::*;

/// A [`Controller`] widget that makes the target acts as a window grab handle.
///
/// On most platforms, click events aren't passed through to the children. As such, it is
/// recommended to only put text or small icons in a window drag handle.
///
/// This controller widget only works for the Windows and Gtk platforms. On any other platform,
/// it does absolutely nothing.
///
/// # Examples
///
/// ```
/// use druid::widget::{WindowDragHandle, Label, Button, Flex};
/// use druid::commands::CLOSE_WINDOW;
///
/// let title_label = WindowDragHandle::new(Label::new("My Application"));
/// let exit_button = Button::new("X").on_click(|ctx, _data : &mut (), _env| ctx.submit_command(CLOSE_WINDOW));
/// let title_bar = Flex::row().with_child(title_label).with_child(exit_button);
/// ```
///
/// # Platform notes
///
/// ## Windows
///
/// Click events aren't passed through to the children.
///
/// ## Gtk
///
/// Click events aren't passed through to the children.
///
/// [`Controller`]: struct.Controller.html
pub struct WindowDragHandle;

impl WindowDragHandle {
    /// Create a new window drag handle, wrapping the provided widget.
    pub fn new<T: Data, W: Widget<T>>(child: W) -> ControllerHost<W, Self> {
        ControllerHost::new(child, WindowDragHandle)
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for WindowDragHandle {
    #[instrument(name = "WindowDragHandle", level = "trace", skip(self, child, ctx, event, data, env))]
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        #[cfg(all(feature = "gtk", target_os = "linux"))]
        if let Event::MouseDown(_) = event {
            ctx.window().begin_move_drag()
        }
        #[cfg(target_os = "windows")]
        if let Event::MouseMove(_) = event {
            ctx.window().handle_titlebar(true)
        }
        child.event(ctx, event, data, env);
    }
}