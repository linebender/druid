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

use std::any::TypeId;
use std::collections::HashMap;
use std::mem;

// Automatically defaults to std::time::Instant on non Wasm platforms
use instant::Instant;

use crate::kurbo::{Point, Rect, Size};
use crate::piet::{Piet, RenderContext};
use crate::shell::{Counter, Cursor, WindowHandle};

use crate::core::{BaseState, CommandQueue, FocusChange};
use crate::widget::LabelText;
use crate::win_handler::RUN_COMMANDS_TOKEN;
use crate::{
    BoxConstraints, Command, Data, Env, Event, EventCtx, InternalEvent, InternalLifeCycle,
    LayoutCtx, LifeCycle, LifeCycleCtx, MenuDesc, PaintCtx, TimerToken, UpdateCtx, Widget,
    WidgetId, WidgetPod, WindowDesc,
};

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

/// Per-window state not owned by user code.
pub struct Window<T> {
    pub(crate) id: WindowId,
    pub(crate) root: WidgetPod<T, Box<dyn Widget<T>>>,
    pub(crate) title: LabelText<T>,
    size: Size,
    pub(crate) menu: Option<MenuDesc<T>>,
    pub(crate) context_menu: Option<MenuDesc<T>>,
    pub(crate) last_anim: Option<Instant>,
    pub(crate) last_mouse_pos: Option<Point>,
    pub(crate) focus: Option<WidgetId>,
    pub(crate) handle: WindowHandle,
    pub(crate) timers: HashMap<TimerToken, WidgetId>,
    // delegate?
}

impl<T> Window<T> {
    pub(crate) fn new(id: WindowId, handle: WindowHandle, desc: WindowDesc<T>) -> Window<T> {
        Window {
            id,
            root: WidgetPod::new(desc.root),
            size: Size::ZERO,
            title: desc.title,
            menu: desc.menu,
            context_menu: None,
            last_anim: None,
            last_mouse_pos: None,
            focus: None,
            handle,
            timers: HashMap::new(),
        }
    }
}

impl<T: Data> Window<T> {
    /// `true` iff any child requested an animation frame during the last `AnimFrame` event.
    pub(crate) fn wants_animation_frame(&self) -> bool {
        self.last_anim.is_some()
    }

    pub(crate) fn focus_chain(&self) -> &[WidgetId] {
        &self.root.state().focus_chain
    }

    /// Returns `true` if the provided widget may be in this window,
    /// but it may also be a false positive.
    /// However when this returns `false` the widget is definitely not in this window.
    pub(crate) fn may_contain_widget(&self, widget_id: WidgetId) -> bool {
        // The bloom filter we're checking can return false positives.
        self.root.state().children.may_contain(&widget_id)
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

    fn post_event_processing(
        &mut self,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
        process_commands: bool,
    ) {
        let base_state = self.root.state();
        // If children are changed during the handling of an event,
        // we need to send RouteWidgetAdded now, so that they are ready for update/layout.
        if base_state.children_changed {
            self.lifecycle(
                queue,
                &LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded),
                data,
                env,
                false,
            );
        }
        // If there are any commands and they should be processed
        if process_commands && !queue.is_empty() {
            // Ask the handler to call us back on idle
            // so we can process them in a new event/update pass.
            if let Some(mut handle) = self.handle.get_idle_handle() {
                handle.schedule_idle(RUN_COMMANDS_TOKEN);
            } else {
                log::error!("failed to get idle handle");
            }
        }
    }

    pub(crate) fn event(
        &mut self,
        queue: &mut CommandQueue,
        event: Event,
        data: &mut T,
        env: &Env,
    ) -> bool {
        match &event {
            Event::MouseDown(e) | Event::MouseUp(e) | Event::MouseMove(e) | Event::Wheel(e) => {
                self.last_mouse_pos = Some(e.pos)
            }
            Event::Internal(InternalEvent::MouseLeave) => self.last_mouse_pos = None,
            _ => (),
        }

        let mut cursor = match event {
            Event::MouseMove(..) => Some(Cursor::Arrow),
            _ => None,
        };

        let event = match event {
            Event::WindowSize(size) => {
                let dpi = f64::from(self.handle.get_dpi());
                let scale = 96.0 / dpi;
                self.size = Size::new(size.width * scale, size.height * scale);
                Event::WindowSize(self.size)
            }
            Event::Timer(token) => {
                if let Some(widget_id) = self.timers.get(&token) {
                    Event::Internal(InternalEvent::RouteTimer(token, *widget_id))
                } else {
                    log::error!("No widget found for timer {:?}", token);
                    return false;
                }
            }
            other => other,
        };

        if let Event::WindowConnected = event {
            self.lifecycle(
                queue,
                &LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded),
                data,
                env,
                false,
            );
        }

        let mut base_state = BaseState::new(self.root.id());
        let is_handled = {
            let mut ctx = EventCtx {
                cursor: &mut cursor,
                command_queue: queue,
                base_state: &mut base_state,
                is_handled: false,
                is_root: true,
                window: &self.handle,
                window_id: self.id,
                focus_widget: self.focus,
                app_data_type: TypeId::of::<T>(),
            };

            self.root.event(&mut ctx, &event, data, env);
            ctx.is_handled
        };

        if let Some(focus_req) = base_state.request_focus.take() {
            let old = self.focus;
            let new = self.widget_for_focus_request(focus_req);
            let event = LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { old, new });
            self.lifecycle(queue, &event, data, env, false);
            self.focus = new;
        }

        if let Some(cursor) = cursor {
            self.handle.set_cursor(&cursor);
        }

        self.post_event_processing(queue, data, env, false);

        //In some platforms, timer tokens are reused. So it is necessary to remove the token from
        //the window's timer map before adding new tokens to it.
        if let Event::Internal(InternalEvent::RouteTimer(token, _)) = event {
            self.timers.remove(&token);
        }

        //If at least one widget requested a timer, add all the requested timers to window's timers map.
        if base_state.request_timer {
            self.timers.extend(base_state.timers);
        }

        is_handled
    }

    pub(crate) fn lifecycle(
        &mut self,
        queue: &mut CommandQueue,
        event: &LifeCycle,
        data: &T,
        env: &Env,
        process_commands: bool,
    ) {
        if let LifeCycle::AnimFrame(_) = event {
            self.do_anim_frame(queue, data, env)
        } else {
            let mut base_state = BaseState::new(self.root.id());
            let mut ctx = LifeCycleCtx {
                command_queue: queue,
                window_id: self.id,
                window: &self.handle,
                base_state: &mut base_state,
            };

            self.root.lifecycle(&mut ctx, event, data, env);
        }

        self.post_event_processing(queue, data, env, process_commands);
    }

    /// AnimFrame has special logic, so we implement it separately.
    fn do_anim_frame(&mut self, queue: &mut CommandQueue, data: &T, env: &Env) {
        let mut base_state = BaseState::new(self.root.id());
        let mut ctx = LifeCycleCtx {
            command_queue: queue,
            window_id: self.id,
            window: &self.handle,
            base_state: &mut base_state,
        };

        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/xi-editor/druid/issues/85 for discussion.
        let now = Instant::now();
        let last = self.last_anim.take();
        let elapsed_ns = last.map(|t| now.duration_since(t).as_nanos()).unwrap_or(0) as u64;

        let event = LifeCycle::AnimFrame(elapsed_ns);
        self.root.lifecycle(&mut ctx, &event, data, env);
        if ctx.base_state.request_anim {
            self.last_anim = Some(now);
        }
    }

    pub(crate) fn update(&mut self, queue: &mut CommandQueue, data: &T, env: &Env) {
        self.update_title(data, env);

        let mut base_state = BaseState::new(self.root.id());
        let mut update_ctx = UpdateCtx {
            base_state: &mut base_state,
            command_queue: queue,
            window: &self.handle,
            window_id: self.id,
        };

        self.root.update(&mut update_ctx, data, env);
        self.post_event_processing(queue, data, env, false);
    }

    pub(crate) fn invalidate_and_finalize(&mut self) {
        if self.root.state().needs_layout {
            self.handle.invalidate();
        } else {
            let invalid = &self.root.state().invalid;
            if !invalid.is_empty() {
                self.handle.invalidate_rect(invalid.to_rect());
            }
        }
    }

    /// Do all the stuff we do in response to a paint call from the system:
    /// layout, send an `AnimFrame` event, and then actually paint.
    pub(crate) fn do_paint(
        &mut self,
        piet: &mut Piet,
        invalid_rect: Rect,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
    ) {
        // FIXME: only do AnimFrame if root has requested_anim?
        self.lifecycle(queue, &LifeCycle::AnimFrame(0), data, env, true);

        if self.root.state().needs_layout {
            self.layout(piet, queue, data, env);
        }

        piet.fill(
            invalid_rect,
            &env.get(crate::theme::WINDOW_BACKGROUND_COLOR),
        );
        self.paint(piet, invalid_rect, data, env);
    }

    fn layout(&mut self, piet: &mut Piet, queue: &mut CommandQueue, data: &T, env: &Env) {
        let mut base_state = BaseState::new(self.root.id());
        let mut layout_ctx = LayoutCtx {
            command_queue: queue,
            base_state: &mut base_state,
            text_factory: piet.text(),
            window_id: self.id,
            window: &self.handle,
            mouse_pos: self.last_mouse_pos,
        };
        let bc = BoxConstraints::tight(self.size);
        let size = self.root.layout(&mut layout_ctx, &bc, data, env);
        self.root.set_layout_rect(
            &mut layout_ctx,
            data,
            env,
            Rect::from_origin_size(Point::ORIGIN, size),
        );
        self.post_event_processing(queue, data, env, true);
    }

    /// only expose `layout` for testing; normally it is called as part of `do_paint`
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(test)]
    pub(crate) fn just_layout(
        &mut self,
        piet: &mut Piet,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
    ) {
        self.layout(piet, queue, data, env)
    }

    fn paint(&mut self, piet: &mut Piet, invalid_rect: Rect, data: &T, env: &Env) {
        let base_state = BaseState::new(self.root.id());
        let mut ctx = PaintCtx {
            render_ctx: piet,
            base_state: &base_state,
            window_id: self.id,
            z_ops: Vec::new(),
            focus_widget: self.focus,
            region: invalid_rect.into(),
            depth: 0,
        };
        ctx.with_child_ctx(invalid_rect, |ctx| self.root.paint(ctx, data, env));

        let mut z_ops = mem::take(&mut ctx.z_ops);
        z_ops.sort_by_key(|k| k.z_index);

        for z_op in z_ops.into_iter() {
            ctx.with_child_ctx(invalid_rect, |ctx| {
                ctx.with_save(|ctx| {
                    ctx.render_ctx.transform(z_op.transform);
                    (z_op.paint_func)(ctx);
                });
            });
        }
    }

    pub(crate) fn update_title(&mut self, data: &T, env: &Env) {
        if self.title.resolve(data, env) {
            self.handle.set_title(self.title.display_text());
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
            FocusChange::Next => self.widget_from_focus_chain(true),
            FocusChange::Previous => self.widget_from_focus_chain(false),
        }
    }

    fn widget_from_focus_chain(&self, forward: bool) -> Option<WidgetId> {
        self.focus.and_then(|focus| {
            self.focus_chain()
                .iter()
                // Find where the focused widget is in the focus chain
                .position(|id| id == &focus)
                .map(|idx| {
                    // Return the id that's next to it in the focus chain
                    let len = self.focus_chain().len();
                    let new_idx = if forward {
                        (idx + 1) % len
                    } else {
                        (idx + len - 1) % len
                    };
                    self.focus_chain()[new_idx]
                })
                .or_else(|| {
                    // If the currently focused widget isn't in the focus chain,
                    // then we'll just return the first/last entry of the chain, if any.
                    if forward {
                        self.focus_chain().first().copied()
                    } else {
                        self.focus_chain().last().copied()
                    }
                })
        })
    }
}

impl WindowId {
    /// Allocate a new, unique window id.
    pub fn next() -> WindowId {
        static WINDOW_COUNTER: Counter = Counter::new();
        WindowId(WINDOW_COUNTER.next())
    }
}
