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

use std::mem;
use std::time::Instant;

use crate::kurbo::{Point, Rect, Size};
use crate::piet::{Piet, RenderContext};
use crate::shell::{Counter, Cursor, WinCtx, WindowHandle};

use crate::core::{BaseState, CommandQueue, FocusChange};
use crate::win_handler::RUN_COMMANDS_TOKEN;
use crate::{
    BoxConstraints, Command, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    LocalizedString, MenuDesc, PaintCtx, UpdateCtx, Widget, WidgetId, WidgetPod,
};

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

/// Internal window state that is waiting for a window handle to show up.
pub(crate) struct PendingWindow<T: Data> {
    root: WidgetPod<T, Box<dyn Widget<T>>>,
    title: LocalizedString<T>,
    menu: Option<MenuDesc<T>>,
}

/// Per-window state not owned by user code.
pub struct Window<T: Data> {
    pub(crate) id: WindowId,
    pub(crate) root: WidgetPod<T, Box<dyn Widget<T>>>,
    pub(crate) title: LocalizedString<T>,
    size: Size,
    pub(crate) menu: Option<MenuDesc<T>>,
    pub(crate) context_menu: Option<MenuDesc<T>>,
    pub(crate) last_anim: Option<Instant>,
    pub(crate) needs_inval: bool,
    pub(crate) children_changed: bool,
    pub(crate) focus: Option<WidgetId>,
    pub(crate) focus_widgets: Vec<WidgetId>,
    pub(crate) handle: WindowHandle,
    // delegate?
}

impl<T: Data> PendingWindow<T> {
    pub(crate) fn new(
        root: impl Widget<T> + 'static,
        title: LocalizedString<T>,
        menu: Option<MenuDesc<T>>,
    ) -> PendingWindow<T> {
        PendingWindow {
            root: WidgetPod::new(Box::new(root)),
            title,
            menu,
        }
    }

    pub(crate) fn into_window(self, id: WindowId, handle: WindowHandle) -> Window<T> {
        let PendingWindow { root, title, menu } = self;
        Window {
            id,
            root,
            size: Size::ZERO,
            title,
            menu,
            context_menu: None,
            last_anim: None,
            needs_inval: false,
            children_changed: false,
            focus: None,
            focus_widgets: Vec::new(),
            handle,
        }
    }
}

impl<T: Data> Window<T> {
    /// `true` iff any child requested an animation frame during the last `AnimFrame` event.
    pub(crate) fn wants_animation_frame(&self) -> bool {
        self.last_anim.is_some()
    }

    pub(crate) fn set_menu(&mut self, mut menu: MenuDesc<T>, data: &T, env: &Env) {
        let platform_menu = menu.build_window_menu(data, env);
        self.handle.set_menu(platform_menu);
        self.menu = Some(menu);
    }

    pub(crate) fn show_context_menu(
        &mut self,
        mut menu: MenuDesc<T>,
        point: Point,
        data: &T,
        env: &Env,
    ) {
        let platform_menu = menu.build_popup_menu(data, env);
        self.handle.show_context_menu(platform_menu, point);
        self.context_menu = Some(menu);
    }

    /// On macos we need to update the global application menu to be the menu
    /// for the current window.
    #[cfg(target_os = "macos")]
    pub(crate) fn macos_update_app_menu(&mut self, data: &T, env: &Env) {
        if let Some(menu) = self.menu.as_mut().map(|m| m.build_window_menu(data, env)) {
            self.handle.set_menu(menu);
        }
    }

    pub(crate) fn event(
        &mut self,
        win_ctx: &mut dyn WinCtx,
        queue: &mut CommandQueue,
        event: Event,
        data: &mut T,
        env: &Env,
    ) -> bool {
        let mut cursor = match event {
            Event::MouseMoved(..) => Some(Cursor::Arrow),
            _ => None,
        };

        let event = match event {
            Event::Size(size) => {
                let dpi = f64::from(self.handle.get_dpi());
                let scale = 96.0 / dpi;
                self.size = Size::new(size.width * scale, size.height * scale);
                Event::Size(self.size)
            }
            other => other,
        };

        let mut base_state = BaseState::new(self.root.id());
        let is_handled = {
            let mut ctx = EventCtx {
                win_ctx,
                cursor: &mut cursor,
                command_queue: queue,
                base_state: &mut base_state,
                is_handled: false,
                is_root: true,
                had_active: self.root.has_active(),
                window: &self.handle,
                window_id: self.id,
                focus_widget: self.focus,
            };

            self.root.event(&mut ctx, &event, data, env);
            ctx.is_handled
        };

        if let Some(focus_req) = base_state.request_focus.take() {
            let old = self.focus;
            let new = self.widget_for_focus_request(focus_req);
            let event = LifeCycle::RouteFocusChanged { old, new };
            self.lifecycle(queue, &event, data, env);
            self.focus = new;
        }

        if let Some(cursor) = cursor {
            win_ctx.set_cursor(&cursor);
        }

        self.needs_inval |= base_state.needs_inval;
        // If children are changed during the handling of an event,
        // we need to send WidgetAdded and Register now, so that they
        // are ready for update/layout.
        if base_state.children_changed {
            self.lifecycle(queue, &LifeCycle::Register, data, env);
        }

        is_handled
    }

    /// Returns `true` if any widget has requested an animation frame
    pub(crate) fn lifecycle(
        &mut self,
        queue: &mut CommandQueue,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        let mut base_state = BaseState::new(self.root.id());
        let mut ctx = LifeCycleCtx {
            command_queue: queue,
            window_id: self.id,
            base_state: &mut base_state,
        };

        if let LifeCycle::AnimFrame(_) = event {
            return self.do_anim_frame(&mut ctx, data, env);
        }

        self.root.lifecycle(&mut ctx, event, data, env);
        self.needs_inval |= ctx.base_state.needs_inval;
        self.children_changed |= ctx.base_state.children_changed;

        if let LifeCycle::Register = event {
            self.focus_widgets = std::mem::take(&mut ctx.base_state.focus_chain);
        }
    }

    /// AnimFrame has special logic, so we implement it separately.
    fn do_anim_frame(&mut self, ctx: &mut LifeCycleCtx, data: &T, env: &Env) {
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/xi-editor/druid/issues/85 for discussion.
        let now = Instant::now();
        let last = self.last_anim.take();
        let elapsed_ns = last.map(|t| now.duration_since(t).as_nanos()).unwrap_or(0) as u64;

        let event = LifeCycle::AnimFrame(elapsed_ns);
        self.root.lifecycle(ctx, &event, data, env);
        if ctx.base_state.request_anim {
            self.last_anim = Some(now);
        }
    }

    pub(crate) fn update(&mut self, win_ctx: &mut dyn WinCtx, data: &T, env: &Env) {
        self.update_title(data, env);

        let mut update_ctx = UpdateCtx {
            text_factory: win_ctx.text_factory(),
            window: &self.handle,
            needs_inval: false,
            children_changed: false,
            window_id: self.id,
            widget_id: self.root.id(),
        };

        self.root.update(&mut update_ctx, data, env);
        self.needs_inval |= update_ctx.needs_inval;
        self.children_changed |= update_ctx.children_changed;
    }

    pub(crate) fn invalidate_and_finalize(
        &mut self,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
    ) {
        if self.needs_inval {
            self.handle.invalidate();
            // TODO: should we just clear this after paint?
            self.needs_inval = false;
        }
        if self.children_changed {
            self.lifecycle(queue, &LifeCycle::Register, data, env);
            self.children_changed = false;
        }
    }

    /// Do all the stuff we do in response to a paint call from the system:
    /// layout, send an `AnimFrame` event, and then actually paint.
    pub(crate) fn do_paint(
        &mut self,
        piet: &mut Piet,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
    ) {
        self.lifecycle(queue, &LifeCycle::AnimFrame(0), data, env);
        self.layout(piet, data, env);
        piet.clear(env.get(crate::theme::WINDOW_BACKGROUND_COLOR));
        self.paint(piet, data, env);

        // If commands were submitted during anim frame, ask the handler
        // to call us back on idle so we can process them in a new event/update pass.
        if !queue.is_empty() {
            if let Some(mut handle) = self.handle.get_idle_handle() {
                handle.schedule_idle(RUN_COMMANDS_TOKEN);
            } else {
                log::error!("failed to get idle handle");
            }
        }
    }

    fn layout(&mut self, piet: &mut Piet, data: &T, env: &Env) {
        let mut layout_ctx = LayoutCtx {
            text_factory: piet.text(),
            window_id: self.id,
        };
        let bc = BoxConstraints::tight(self.size);
        let size = self.root.layout(&mut layout_ctx, &bc, data, env);
        self.root
            .set_layout_rect(Rect::from_origin_size(Point::ORIGIN, size));
    }

    /// only expose `layout` for testing; normally it is called as part of `do_paint`
    #[cfg(test)]
    pub(crate) fn just_layout(&mut self, piet: &mut Piet, data: &T, env: &Env) {
        self.layout(piet, data, env)
    }

    fn paint(&mut self, piet: &mut Piet, data: &T, env: &Env) {
        let base_state = BaseState::new(self.root.id());
        let mut paint_ctx = PaintCtx {
            render_ctx: piet,
            base_state: &base_state,
            window_id: self.id,
            focus_widget: self.focus,
            region: Rect::ZERO.into(),
        };
        let visible = Rect::from_origin_size(Point::ZERO, self.size);
        paint_ctx.with_child_ctx(visible, |ctx| self.root.paint(ctx, data, env));

        paint_ctx.z_ops.sort_by_key(|k| k.z_index);

        let z_ops = mem::replace(&mut paint_ctx.z_ops, Vec::new());
        for z_op in z_ops.into_iter() {
            paint_ctx.with_child_ctx(visible, |ctx| {
                if let Err(e) = ctx.render_ctx.save() {
                    log::error!("saving render context failed: {:?}", e);
                    return;
                }

                ctx.render_ctx.transform(z_op.transform);
                (z_op.paint_func)(ctx);

                if let Err(e) = ctx.render_ctx.restore() {
                    log::error!("restoring render context failed: {:?}", e);
                }
            });
        }
    }

    pub(crate) fn update_title(&mut self, data: &T, env: &Env) {
        if self.title.resolve(data, env) {
            self.handle.set_title(self.title.localized_str());
        }
    }

    pub(crate) fn get_menu_cmd(&self, cmd_id: u32) -> Option<Command> {
        self.context_menu
            .as_ref()
            .and_then(|m| m.command_for_id(cmd_id))
            .or_else(|| self.menu.as_ref().and_then(|m| m.command_for_id(cmd_id)))
    }

    fn widget_for_focus_request(&self, focus: FocusChange) -> Option<WidgetId> {
        match focus {
            FocusChange::Resign => None,
            FocusChange::Focus(id) => Some(id),
            FocusChange::Next => self
                .focus
                .and_then(|id| self.focus_widgets.iter().position(|i| i == &id))
                .map(|idx| {
                    let next_idx = (idx + 1) % self.focus_widgets.len();
                    self.focus_widgets[next_idx]
                }),
            FocusChange::Previous => self
                .focus
                .and_then(|id| self.focus_widgets.iter().position(|i| i == &id))
                .map(|idx| {
                    let len = self.focus_widgets.len();
                    let prev_idx = (idx + len - 1) % len;
                    self.focus_widgets[prev_idx]
                }),
        }
    }
}

impl WindowId {
    /// Allocate a new, unique window id.
    pub fn next() -> WindowId {
        static WINDOW_COUNTER: Counter = Counter::new();
        WindowId(WINDOW_COUNTER.next())
    }
}
