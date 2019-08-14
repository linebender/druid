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

//! The implementation of the WinHandler trait (druid-shell integration).

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use crate::kurbo::{Size, Vec2};

use crate::piet::{Color, Piet, RenderContext};

use druid_shell::window::{Cursor, WinCtx, WinHandler, WindowHandle};

use crate::{
    Data, Env, Event, EventCtxRoot, KeyEvent, KeyModifiers, LayoutCtx, LayoutCtxRoot, MouseEvent,
    PaintCtx, PaintCtxRoot, RootWidget, TimerToken, UpdateCtxRoot, WheelEvent, WindowId,
};

// TODO: this should come from the theme.
const BACKGROUND_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

/// The struct implements the druid-shell `WinHandler` trait
struct DruidHandler<T: Data, R: RootWidget<T>> {
    app_state: Rc<RefCell<AppState<T, R>>>,
    window_id: WindowId,
}

/// State shared by all windows in the UI.
struct AppState<T: Data, R: RootWidget<T>> {
    window_state: HashMap<WindowId, WindowState>,
    env: Env,
    data: T,
    root: R,
}

/// Per-window state not owned by user code.
pub(crate) struct WindowState {
    pub(crate) handle: WindowHandle,
    prev_paint_time: Option<Instant>,
}

impl<T: Data + 'static, R: RootWidget<T> + 'static> WinHandler for DruidHandler<T, R> {
    fn connect(&mut self, handle: &WindowHandle) {
        let state = WindowState {
            handle: handle.clone(),
            prev_paint_time: None,
        };
        self.app_state
            .borrow_mut()
            .window_state
            .insert(self.window_id, state);
    }

    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        self.app_state.borrow_mut().paint(self.window_id, piet, ctx)
    }

    fn size(&mut self, width: u32, height: u32, ctx: &mut dyn WinCtx) {
        let dpi = self
            .app_state
            .borrow()
            .window_state
            .get(&self.window_id)
            .unwrap()
            .handle
            .get_dpi() as f64;
        let scale = 96.0 / dpi;
        let event = Event::Size(Size::new(width as f64 * scale, height as f64 * scale));
        self.do_event(event, ctx);
    }

    fn mouse_down(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        // TODO: double-click detection (or is this done in druid-shell?)
        let event = Event::MouseDown(event.clone());
        self.do_event(event, ctx);
    }

    fn mouse_up(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        let event = Event::MouseUp(event.clone());
        self.do_event(event, ctx);
    }

    fn mouse_move(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        let event = Event::MouseMoved(event.clone());
        self.do_event(event, ctx);
    }

    fn key_down(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) -> bool {
        self.do_event(Event::KeyDown(event), ctx)
    }

    fn key_up(&mut self, event: KeyEvent, ctx: &mut dyn WinCtx) {
        self.do_event(Event::KeyUp(event), ctx);
    }

    fn wheel(&mut self, delta: Vec2, mods: KeyModifiers, ctx: &mut dyn WinCtx) {
        let event = Event::Wheel(WheelEvent { delta, mods });
        self.do_event(event, ctx);
    }

    fn timer(&mut self, token: TimerToken, ctx: &mut dyn WinCtx) {
        self.do_event(Event::Timer(token), ctx);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T: Data, R: RootWidget<T>> DruidHandler<T, R> {
    /// Send an event to the widget hierarchy.
    ///
    /// Returns `true` if the event produced an action.
    ///
    /// This is principally because in certain cases (such as keydown on Windows)
    /// the OS needs to know if an event was handled.
    fn do_event(&mut self, event: Event, win_ctx: &mut dyn WinCtx) -> bool {
        let mut state = self.app_state.borrow_mut();
        let (is_handled, dirty, anim) = state.do_event_inner(self.window_id, event, win_ctx);
        if dirty || anim {
            win_ctx.invalidate();
        }
        is_handled
    }
}

impl<T: Data, R: RootWidget<T>> AppState<T, R> {
    fn paint(&mut self, window_id: WindowId, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/xi-editor/druid/issues/85 for discussion.
        let this_paint_time = Instant::now();
        let prev_paint_time = self.window_state.get(&window_id).unwrap().prev_paint_time;
        let interval = if let Some(last) = prev_paint_time {
            let duration = this_paint_time.duration_since(last);
            1_000_000_000 * duration.as_secs() + (duration.subsec_nanos() as u64)
        } else {
            0
        };
        let anim_frame_event = Event::AnimFrame(interval);
        let (_, _, request_anim) = self.do_event_inner(window_id, anim_frame_event, ctx);
        let prev = if request_anim {
            Some(this_paint_time)
        } else {
            None
        };
        self.window_state
            .get_mut(&window_id)
            .unwrap()
            .prev_paint_time = prev;
        let text_factory = piet.text();
        let mut layout_ctx = LayoutCtxRoot(LayoutCtx {
            text_factory,
            window_id,
        });
        self.root.layout(&mut layout_ctx, &self.data, &self.env);
        piet.clear(BACKGROUND_COLOR);
        let mut paint_ctx = PaintCtxRoot(PaintCtx {
            render_ctx: piet,
            window_id,
        });
        self.root.paint(&mut paint_ctx, &self.data, &self.env);
        request_anim
    }
}

impl<T: Data, R: RootWidget<T>> AppState<T, R> {
    /// Send an event to the widget hierarchy.
    ///
    /// Returns three flags. The first is true if the event was handled. The
    /// second is true if invalidation is requested. The third is true if an
    /// animation frame is requested.
    fn do_event_inner(
        &mut self,
        window_id: WindowId,
        event: Event,
        win_ctx: &mut dyn WinCtx,
    ) -> (bool, bool, bool) {
        // should there be a root base state persisting in the ui state instead?
        let mut cursor = match event {
            Event::MouseMoved(..) => Some(Cursor::Arrow),
            _ => None,
        };
        let mut ctx = EventCtxRoot {
            win_ctx,
            cursor: &mut cursor,
            window: &self.window_state.get(&window_id).unwrap().handle,
            base_state: Default::default(),
            is_handled: false,
            window_id,
        };
        self.root.event(&event, &mut ctx, &mut self.data, &self.env);

        let is_handled = ctx.is_handled;
        if ctx.base_state.request_focus {
            let focus_event = Event::FocusChanged(true);
            self.root
                .event(&focus_event, &mut ctx, &mut self.data, &self.env);
        }
        let needs_inval = ctx.base_state.needs_inval;
        let request_anim = ctx.base_state.request_anim;
        if let Some(cursor) = cursor {
            win_ctx.set_cursor(&cursor);
        }

        let mut update_ctx = UpdateCtxRoot {
            text_factory: win_ctx.text_factory(),
            window_state: &self.window_state,
            originating_window: window_id,
            needs_inval: false,
        };
        // Note: we probably want to aggregate updates so there's only one after
        // a burst of events.
        self.root.update(&mut update_ctx, &self.data, &self.env);
        // TODO: process actions (maybe?)

        let dirty = needs_inval || update_ctx.needs_inval;
        (is_handled, dirty, request_anim)
    }
}
