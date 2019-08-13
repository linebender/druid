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
use druid_shell::WindowBuilder;

use crate::{
    Data, Env, Event, EventCtxRoot, KeyEvent, KeyModifiers, LayoutCtx, LayoutCtxRoot, MouseEvent,
    PaintCtx, PaintCtxRoot, RootWidget, TimerToken, UpdateCtxRoot, WheelEvent, WindowId,
};

use crate::theme;

// TODO: this should come from the theme.
const BACKGROUND_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

/// The struct implements the druid-shell `WinHandler` trait.
///
/// This is something of an internal detail and possibly we don't want to surface
/// it publicly.
pub struct DruidHandler<T: Data> {
    app_state: Rc<RefCell<AppState<T>>>,
    window_id: WindowId,
}

/// State shared by all windows in the UI.
pub(crate) struct AppState<T: Data> {
    window_state: HashMap<WindowId, WindowState>,
    env: Env,
    data: T,
    root: Box<dyn RootWidget<T>>,
}

/// Per-window state not owned by user code.
pub(crate) struct WindowState {
    pub(crate) handle: WindowHandle,
    prev_paint_time: Option<Instant>,
}

impl<T: Data + 'static> WinHandler for DruidHandler<T> {
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

impl<T: Data + 'static> DruidHandler<T> {
    /// Create a new handler for the first window in the app.
    pub fn new(
        root: impl RootWidget<T> + 'static,
        data: T,
        window_id: WindowId,
    ) -> DruidHandler<T> {
        let app_state = AppState {
            root: Box::new(root),
            env: theme::init(),
            data,
            window_state: HashMap::new(),
        };
        DruidHandler {
            app_state: Rc::new(RefCell::new(app_state)),
            window_id,
        }
    }

    /// Note: the root widget doesn't go in here, because it gets added to the
    /// app state.
    pub(crate) fn new_shared(
        app_state: Rc<RefCell<AppState<T>>>,
        window_id: WindowId,
    ) -> DruidHandler<T> {
        DruidHandler {
            app_state,
            window_id,
        }
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns `true` if the event produced an action.
    ///
    /// This is principally because in certain cases (such as keydown on Windows)
    /// the OS needs to know if an event was handled.
    fn do_event(&mut self, event: Event, win_ctx: &mut dyn WinCtx) -> bool {
        let mut new_window_queue = Vec::new();
        let (is_handled, dirty, anim) = self.app_state.borrow_mut().do_event_inner(
            self.window_id,
            event,
            win_ctx,
            &mut new_window_queue,
        );
        self.create_new_windows(new_window_queue);
        if dirty || anim {
            win_ctx.invalidate();
        }
        is_handled
    }

    fn create_new_windows(&mut self, queue: Vec<(WindowId, WindowBuilder)>) {
        for (window_id, mut builder) in queue {
            // TODO: if we set the handler closer to build time, we can probably get
            // rid of the `app_state` field.
            let handler = DruidHandler::new_shared(self.app_state.clone(), window_id);
            builder.set_handler(Box::new(handler));
            let result = builder.build();
            match result {
                Ok(handle) => {
                    handle.show();
                    // TODO: this newtype wrapping should be elsewhere.
                    let handle = druid_shell::window::WindowHandle { inner: handle };
                    let window_state = WindowState {
                        handle,
                        prev_paint_time: None,
                    };
                    self.app_state
                        .borrow_mut()
                        .window_state
                        .insert(window_id, window_state);
                }
                Err(e) => println!("Error building window: {:?}", e),
            }
        }
    }
}

// Note: we could minimize cloning of the self_ref by making these methods on
// `DruidHandler` instead, but I'm not sure that refactor is worth it.
impl<T: Data> AppState<T> {
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
        let mut new_window_queue = Vec::new();
        let (_, _, request_anim) =
            self.do_event_inner(window_id, anim_frame_event, ctx, &mut new_window_queue);
        debug_assert!(
            new_window_queue.is_empty(),
            "Adding windows from AnimFrame event not supported"
        );
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
        new_window_queue: &mut Vec<(WindowId, WindowBuilder)>,
    ) -> (bool, bool, bool) {
        // should there be a root base state persisting in the ui state instead?
        let mut cursor = match event {
            Event::MouseMoved(..) => Some(Cursor::Arrow),
            _ => None,
        };
        let mut ctx = EventCtxRoot {
            win_ctx,
            cursor: &mut cursor,
            window_state: &mut self.window_state,
            new_window_queue,
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
