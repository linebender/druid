// Copyright 2019 The Druid Authors.
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

use std::collections::{HashMap, VecDeque};
use std::mem;
use tracing::{error, info, info_span};

// Automatically defaults to std::time::Instant on non Wasm platforms
use instant::Instant;

use crate::piet::{Color, Piet, RenderContext};
use crate::shell::{Counter, Cursor, Region, WindowHandle};

use crate::app::{PendingWindow, WindowSizePolicy};
use crate::contexts::ContextState;
use crate::core::{CommandQueue, FocusChange, WidgetState};
use crate::util::ExtendDrain;
use crate::widget::LabelText;
use crate::win_handler::RUN_COMMANDS_TOKEN;
use crate::{
    BoxConstraints, Command, Data, Env, Event, EventCtx, ExtEventSink, Handled, InternalEvent,
    InternalLifeCycle, LayoutCtx, LifeCycle, LifeCycleCtx, MenuDesc, PaintCtx, Point, Size,
    TimerToken, UpdateCtx, Widget, WidgetId, WidgetPod,
};

/// FIXME: Replace usage with Color::TRANSPARENT on next Piet release
const TRANSPARENT: Color = Color::rgba8(0, 0, 0, 0);

/// A unique identifier for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

/// Per-window state not owned by user code.
pub struct Window<T> {
    pub(crate) id: WindowId,
    pub(crate) root: WidgetPod<T, Box<dyn Widget<T>>>,
    pub(crate) title: LabelText<T>,
    size_policy: WindowSizePolicy,
    size: Size,
    invalid: Region,
    pub(crate) menu: Option<MenuDesc<T>>,
    pub(crate) context_menu: Option<MenuDesc<T>>,
    // This will be `Some` whenever the most recently displayed frame was an animation frame.
    pub(crate) last_anim: Option<Instant>,
    pub(crate) last_mouse_pos: Option<Point>,
    pub(crate) focus: Option<WidgetId>,
    pub(crate) handle: WindowHandle,
    pub(crate) timers: HashMap<TimerToken, WidgetId>,
    pub(crate) transparent: bool,
    ext_handle: ExtEventSink,
    // delegate?
}

impl<T> Window<T> {
    pub(crate) fn new(
        id: WindowId,
        handle: WindowHandle,
        pending: PendingWindow<T>,
        ext_handle: ExtEventSink,
    ) -> Window<T> {
        Window {
            id,
            root: WidgetPod::new(pending.root),
            size_policy: pending.size_policy,
            size: Size::ZERO,
            invalid: Region::EMPTY,
            title: pending.title,
            transparent: pending.transparent,
            menu: pending.menu,
            context_menu: None,
            last_anim: None,
            last_mouse_pos: None,
            focus: None,
            handle,
            timers: HashMap::new(),
            ext_handle,
        }
    }
}

impl<T: Data> Window<T> {
    /// `true` iff any child requested an animation frame since the last `AnimFrame` event.
    pub(crate) fn wants_animation_frame(&self) -> bool {
        self.root.state().request_anim
    }

    pub(crate) fn focus_chain(&self) -> &[WidgetId] {
        &self.root.state().focus_chain
    }

    /// Returns `true` if the provided widget may be in this window,
    /// but it may also be a false positive.
    /// However when this returns `false` the widget is definitely not in this window.
    pub(crate) fn may_contain_widget(&self, widget_id: WidgetId) -> bool {
        // The bloom filter we're checking can return false positives.
        widget_id == self.root.id() || self.root.state().children.may_contain(&widget_id)
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
        widget_state: &mut WidgetState,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
        process_commands: bool,
    ) {
        // If children are changed during the handling of an event,
        // we need to send RouteWidgetAdded now, so that they are ready for update/layout.
        if widget_state.children_changed {
            self.lifecycle(
                queue,
                &LifeCycle::Internal(InternalLifeCycle::RouteWidgetAdded),
                data,
                env,
                false,
            );
        }
        // Add all the requested timers to the window's timers map.
        self.timers.extend_drain(&mut widget_state.timers);

        // If we need a new paint pass, make sure druid-shell knows it.
        if self.wants_animation_frame() {
            self.handle.request_anim_frame();
        }
        self.invalid.union_with(&widget_state.invalid);

        // If there are any commands and they should be processed
        if process_commands && !queue.is_empty() {
            // Ask the handler to call us back on idle
            // so we can process them in a new event/update pass.
            if let Some(mut handle) = self.handle.get_idle_handle() {
                handle.schedule_idle(RUN_COMMANDS_TOKEN);
            } else {
                error!("failed to get idle handle");
            }
        }
    }

    pub(crate) fn event(
        &mut self,
        queue: &mut CommandQueue,
        event: Event,
        data: &mut T,
        env: &Env,
    ) -> Handled {
        match &event {
            Event::WindowSize(size) => self.size = *size,
            Event::MouseDown(e) | Event::MouseUp(e) | Event::MouseMove(e) | Event::Wheel(e) => {
                self.last_mouse_pos = Some(e.pos)
            }
            Event::Internal(InternalEvent::MouseLeave) => self.last_mouse_pos = None,
            _ => (),
        }

        let event = match event {
            Event::Timer(token) => {
                if let Some(widget_id) = self.timers.get(&token) {
                    Event::Internal(InternalEvent::RouteTimer(token, *widget_id))
                } else {
                    error!("No widget found for timer {:?}", token);
                    return Handled::No;
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

        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size));
        let is_handled = {
            let mut state =
                ContextState::new::<T>(queue, &self.ext_handle, &self.handle, self.id, self.focus);
            let mut notifications = VecDeque::new();
            let mut ctx = EventCtx {
                state: &mut state,
                notifications: &mut notifications,
                widget_state: &mut widget_state,
                is_handled: false,
                is_root: true,
            };

            {
                let _span = info_span!("event");
                let _span = _span.enter();
                self.root.event(&mut ctx, &event, data, env);
            }

            if !ctx.notifications.is_empty() {
                info!("{} unhandled notifications:", ctx.notifications.len());
                for (i, n) in ctx.notifications.iter().enumerate() {
                    info!("{}: {:?}", i, n);
                }
            }
            Handled::from(ctx.is_handled)
        };

        // Clean up the timer token and do it immediately after the event handling
        // because the token may be reused and re-added in a lifecycle pass below.
        if let Event::Internal(InternalEvent::RouteTimer(token, _)) = event {
            self.timers.remove(&token);
        }

        if let Some(focus_req) = widget_state.request_focus.take() {
            let old = self.focus;
            let new = self.widget_for_focus_request(focus_req);
            // Only send RouteFocusChanged in case there's actual change
            if old != new {
                let event = LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { old, new });
                self.lifecycle(queue, &event, data, env, false);
                self.focus = new;
            }
        }

        if let Some(cursor) = &widget_state.cursor {
            self.handle.set_cursor(&cursor);
        } else if matches!(
            event,
            Event::MouseMove(..) | Event::Internal(InternalEvent::MouseLeave)
        ) {
            self.handle.set_cursor(&Cursor::Arrow);
        }

        if matches!(
            (event, self.size_policy),
            (Event::WindowSize(_), WindowSizePolicy::Content)
        ) {
            // Because our initial size can be zero, the window system won't ask us to paint.
            // So layout ourselves and hopefully we resize
            self.layout(queue, data, env);
        }

        self.post_event_processing(&mut widget_state, queue, data, env, false);

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
        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size));
        let mut state =
            ContextState::new::<T>(queue, &self.ext_handle, &self.handle, self.id, self.focus);
        let mut ctx = LifeCycleCtx {
            state: &mut state,
            widget_state: &mut widget_state,
        };

        {
            let _span = info_span!("lifecycle");
            let _span = _span.enter();
            self.root.lifecycle(&mut ctx, event, data, env);
        }

        self.post_event_processing(&mut widget_state, queue, data, env, process_commands);
    }

    pub(crate) fn update(&mut self, queue: &mut CommandQueue, data: &T, env: &Env) {
        self.update_title(data, env);

        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size));
        let mut state =
            ContextState::new::<T>(queue, &self.ext_handle, &self.handle, self.id, self.focus);
        let mut update_ctx = UpdateCtx {
            widget_state: &mut widget_state,
            state: &mut state,
            prev_env: None,
            env,
        };

        {
            let _span = info_span!("update");
            let _span = _span.enter();
            self.root.update(&mut update_ctx, data, env);
        }

        if let Some(cursor) = &widget_state.cursor {
            self.handle.set_cursor(cursor);
        }

        self.post_event_processing(&mut widget_state, queue, data, env, false);
    }

    pub(crate) fn invalidate_and_finalize(&mut self) {
        if self.root.state().needs_layout {
            self.handle.invalidate();
        } else {
            for rect in self.invalid.rects() {
                self.handle.invalidate_rect(*rect);
            }
        }
        self.invalid.clear();
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn invalid(&self) -> &Region {
        &self.invalid
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn invalid_mut(&mut self) -> &mut Region {
        &mut self.invalid
    }

    /// Get ready for painting, by doing layout and sending an `AnimFrame` event.
    pub(crate) fn prepare_paint(&mut self, queue: &mut CommandQueue, data: &mut T, env: &Env) {
        let now = Instant::now();
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/linebender/druid/issues/85 for discussion.
        let last = self.last_anim.take();
        let elapsed_ns = last.map(|t| now.duration_since(t).as_nanos()).unwrap_or(0) as u64;

        if self.wants_animation_frame() {
            self.event(queue, Event::AnimFrame(elapsed_ns), data, env);
            self.last_anim = Some(now);
        }
    }

    pub(crate) fn do_paint(
        &mut self,
        piet: &mut Piet,
        invalid: &Region,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
    ) {
        if self.root.state().needs_layout {
            self.layout(queue, data, env);
        }

        piet.fill(
            invalid.bounding_box(),
            &(if self.transparent {
                TRANSPARENT
            } else {
                env.get(crate::theme::WINDOW_BACKGROUND_COLOR)
            }),
        );
        self.paint(piet, invalid, queue, data, env);
    }

    fn layout(&mut self, queue: &mut CommandQueue, data: &T, env: &Env) {
        let mut widget_state = WidgetState::new(self.root.id(), Some(self.size));
        let mut state =
            ContextState::new::<T>(queue, &self.ext_handle, &self.handle, self.id, self.focus);
        let mut layout_ctx = LayoutCtx {
            state: &mut state,
            widget_state: &mut widget_state,
            mouse_pos: self.last_mouse_pos,
        };
        let bc = match self.size_policy {
            WindowSizePolicy::User => BoxConstraints::tight(self.size),
            WindowSizePolicy::Content => BoxConstraints::UNBOUNDED,
        };

        let content_size = {
            let _span = info_span!("layout");
            let _span = _span.enter();
            self.root.layout(&mut layout_ctx, &bc, data, env)
        };

        if let WindowSizePolicy::Content = self.size_policy {
            let insets = self.handle.content_insets();
            let full_size = (content_size.to_rect() + insets).size();
            if self.size != full_size {
                self.size = full_size;
                self.handle.set_size(full_size)
            }
        }
        self.root
            .set_origin(&mut layout_ctx, data, env, Point::ORIGIN);
        self.lifecycle(
            queue,
            &LifeCycle::Internal(InternalLifeCycle::ParentWindowOrigin),
            data,
            env,
            false,
        );
        self.post_event_processing(&mut widget_state, queue, data, env, true);
    }

    /// only expose `layout` for testing; normally it is called as part of `do_paint`
    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(test)]
    pub(crate) fn just_layout(&mut self, queue: &mut CommandQueue, data: &T, env: &Env) {
        self.layout(queue, data, env)
    }

    fn paint(
        &mut self,
        piet: &mut Piet,
        invalid: &Region,
        queue: &mut CommandQueue,
        data: &T,
        env: &Env,
    ) {
        let widget_state = WidgetState::new(self.root.id(), Some(self.size));
        let mut state =
            ContextState::new::<T>(queue, &self.ext_handle, &self.handle, self.id, self.focus);
        let mut ctx = PaintCtx {
            render_ctx: piet,
            state: &mut state,
            widget_state: &widget_state,
            z_ops: Vec::new(),
            region: invalid.clone(),
            depth: 0,
        };

        let root = &mut self.root;
        info_span!("paint").in_scope(|| {
            ctx.with_child_ctx(invalid.clone(), |ctx| root.paint_raw(ctx, data, env));
        });

        let mut z_ops = mem::take(&mut ctx.z_ops);
        z_ops.sort_by_key(|k| k.z_index);

        for z_op in z_ops.into_iter() {
            ctx.with_child_ctx(invalid.clone(), |ctx| {
                ctx.with_save(|ctx| {
                    ctx.render_ctx.transform(z_op.transform);
                    (z_op.paint_func)(ctx);
                });
            });
        }

        if self.wants_animation_frame() {
            self.handle.request_anim_frame();
        }
    }

    pub(crate) fn update_title(&mut self, data: &T, env: &Env) {
        if self.title.resolve(data, env) {
            self.handle.set_title(&self.title.display_text());
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
