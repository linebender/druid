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
use std::rc::{Rc, Weak};
use std::time::Instant;

use log::warn;

use crate::kurbo::{Size, Vec2};

use crate::piet::{Color, Piet, RenderContext};

use druid_shell::window::{Cursor, WinCtx, WinHandler, WindowHandle};

use crate::window::Window;
use crate::{
    BaseState, Command, Data, Env, Event, EventCtx, KeyEvent, KeyModifiers, LayoutCtx, MouseEvent,
    PaintCtx, TimerToken, UpdateCtx, WheelEvent, WindowId,
};

// TODO: this should come from the theme.
const BACKGROUND_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

/// The struct implements the druid-shell `WinHandler` trait.
///
/// One `DruidHandler` exists per window.
///
/// This is something of an internal detail and possibly we don't want to surface
/// it publicly.
pub struct DruidHandler<T: Data> {
    /// The shared app state
    app_state: Rc<RefCell<AppState<T>>>,
    /// The id for the currenet window.
    window_id: WindowId,
}

/// State shared by all windows in the UI.
pub(crate) struct AppState<T: Data> {
    self_ref: Weak<RefCell<AppState<T>>>,
    command_queue: Vec<Command>,
    windows: Windows<T>,
    pub(crate) env: Env,
    pub(crate) data: T,
}

/// All active windows.
struct Windows<T: Data> {
    windows: HashMap<WindowId, Window<T>>,
    state: HashMap<WindowId, WindowState>,
}

/// Per-window state not owned by user code.
pub(crate) struct WindowState {
    pub(crate) handle: WindowHandle,
    prev_paint_time: Option<Instant>,
}

/// Everything required for a window to handle an event.
struct WindowCtx<'a, T: Data> {
    window_id: WindowId,
    window: &'a mut Window<T>,
    state: &'a mut WindowState,
    command_queue: &'a mut Vec<Command>,
    data: &'a mut T,
    env: &'a Env,
}

impl<T: Data> Windows<T> {
    fn connect(&mut self, id: WindowId, handle: WindowHandle) {
        let state = WindowState {
            handle,
            prev_paint_time: None,
        };
        self.state.insert(id, state);
    }

    fn add(&mut self, id: WindowId, window: Window<T>) {
        self.windows.insert(id, window);
    }

    fn get<'a>(
        &'a mut self,
        window_id: WindowId,
        command_queue: &'a mut Vec<Command>,
        data: &'a mut T,
        env: &'a Env,
    ) -> Option<WindowCtx<'a, T>> {
        let state = self.state.get_mut(&window_id);
        let window = self.windows.get_mut(&window_id);

        match (state, window) {
            (Some(state), Some(window)) => {
                return Some(WindowCtx {
                    window_id,
                    window,
                    state,
                    command_queue,
                    data,
                    env,
                })
            }
            (None, Some(_)) => warn!("missing window for id {:?}", window_id),
            (Some(_), None) => warn!("missing state for window id {:?}", window_id),
            (None, None) => warn!("unknown window {:?}", window_id),
        }
        None
    }
}

impl<'a, T: Data> WindowCtx<'a, T> {
    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        let request_anim = self.send_anim_frame(ctx);
        self.send_layout(piet);
        piet.clear(BACKGROUND_COLOR);
        self.send_paint(piet);
        request_anim
    }

    fn send_anim_frame(&mut self, ctx: &mut dyn WinCtx) -> bool {
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/xi-editor/druid/issues/85 for discussion.
        let this_paint_time = Instant::now();
        let prev_paint_time = self.state.prev_paint_time;
        let interval = if let Some(last) = prev_paint_time {
            let duration = this_paint_time.duration_since(last);
            1_000_000_000 * duration.as_secs() + (duration.subsec_nanos() as u64)
        } else {
            0
        };
        let anim_frame_event = Event::AnimFrame(interval);
        let (_, _, request_anim) = self.do_event_inner(anim_frame_event, ctx);
        let prev = if request_anim {
            Some(this_paint_time)
        } else {
            None
        };
        self.state.prev_paint_time = prev;
        request_anim
    }

    fn send_layout(&mut self, piet: &mut Piet) {
        let mut layout_ctx = LayoutCtx {
            text_factory: piet.text(),
            window_id: self.window_id,
        };
        self.window.layout(&mut layout_ctx, self.data, self.env);
    }

    fn send_paint(&mut self, piet: &mut Piet) {
        let mut paint_ctx = PaintCtx {
            render_ctx: piet,
            window_id: self.window_id,
        };
        self.window.paint(&mut paint_ctx, self.data, self.env);
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns three flags. The first is true if the event was handled. The
    /// second is true if invalidation is requested. The third is true if an
    /// animation frame is requested.
    fn do_event_inner(&mut self, event: Event, win_ctx: &mut dyn WinCtx) -> (bool, bool, bool) {
        // should there be a root base state persisting in the ui state instead?
        let mut cursor = match event {
            Event::MouseMoved(..) => Some(Cursor::Arrow),
            _ => None,
        };

        let mut base_state = BaseState::default();
        let mut ctx = EventCtx {
            win_ctx,
            cursor: &mut cursor,
            command_queue: self.command_queue,
            //window_state: &mut self.window_state,
            //new_window_queue,
            base_state: &mut base_state,
            is_handled: false,
            is_root: true,
            had_active: self.window.root.state.has_active,
            window: &self.state.handle,
            window_id: self.window_id,
        };
        self.window.event(&event, &mut ctx, self.data, self.env);

        let is_handled = ctx.is_handled;
        if ctx.base_state.request_focus {
            let focus_event = Event::FocusChanged(true);
            self.window
                .event(&focus_event, &mut ctx, self.data, self.env);
        }
        let needs_inval = ctx.base_state.needs_inval;
        let request_anim = ctx.base_state.request_anim;
        if let Some(cursor) = cursor {
            win_ctx.set_cursor(&cursor);
        }

        let mut update_ctx = UpdateCtx {
            text_factory: win_ctx.text_factory(),
            window: &self.state.handle,
            window_id: self.window_id,
            needs_inval: false,
        };
        // Note: we probably want to aggregate updates so there's only one after
        // a burst of events.
        self.window.update(&mut update_ctx, self.data, self.env);
        // TODO: process actions (maybe?)

        let dirty = needs_inval || update_ctx.needs_inval;
        (is_handled, dirty, request_anim)
    }
}

// Note: we could minimize cloning of the self_ref by making these methods on
// `DruidHandler` instead, but I'm not sure that refactor is worth it.
impl<T: Data> AppState<T> {
    pub(crate) fn new(data: T, env: Env) -> Rc<RefCell<Self>> {
        let slf = Rc::new(RefCell::new(AppState {
            self_ref: Weak::new(),
            command_queue: Vec::new(),
            data,
            env,
            windows: Windows::default(),
        }));

        let self_ref = Rc::downgrade(&slf);
        slf.borrow_mut().self_ref = self_ref;
        slf
    }

    pub(crate) fn add_window(&mut self, id: WindowId, window: Window<T>) {
        self.windows.add(id, window);
    }

    fn window_ctx<'a>(&'a mut self, window_id: WindowId) -> Option<WindowCtx<'a, T>> {
        let AppState {
            ref mut command_queue,
            ref mut windows,
            ref mut data,
            ref env,
            ..
        } = self;
        windows.get(window_id, command_queue, data, env)
    }

    fn paint(&mut self, window_id: WindowId, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        self.window_ctx(window_id)
            .map(|mut win| win.paint(piet, ctx))
            .unwrap_or(false)
    }

    fn do_event(&mut self, id: WindowId, event: Event, win_ctx: &mut dyn WinCtx) -> bool {
        self.window_ctx(id)
            .map(|mut win| {
                let (is_handled, dirty, anim) = win.do_event_inner(event, win_ctx);
                if dirty || anim {
                    win_ctx.invalidate();
                }
                is_handled
            })
            .unwrap_or(false)
    }
}

impl<T: Data + 'static> DruidHandler<T> {
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
        self.app_state
            .borrow_mut()
            .do_event(self.window_id, event, win_ctx)
    }

    //fn create_new_windows(&mut self, queue: Vec<(WindowId, WindowBuilder)>) {
    //for (window_id, mut builder) in queue {
    //// TODO: if we set the handler closer to build time, we can probably get
    //// rid of the `app_state` field.
    //let handler = DruidHandler::new_shared(self.app_state.clone(), window_id);
    //builder.set_handler(Box::new(handler));
    //let result = builder.build();
    //match result {
    //Ok(handle) => {
    //handle.show();
    //// TODO: this newtype wrapping should be elsewhere.
    //let handle = druid_shell::window::WindowHandle { inner: handle };
    //let window_state = WindowState {
    //handle,
    //prev_paint_time: None,
    //};
    //self.app_state
    //.borrow_mut()
    //.window_state
    //.insert(window_id, window_state);
    //}
    //Err(e) => println!("Error building window: {:?}", e),
    //}
    //}
    //}
}

impl<T: Data + 'static> WinHandler for DruidHandler<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.app_state
            .borrow_mut()
            .windows
            .connect(self.window_id, handle.clone());
    }

    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        self.app_state.borrow_mut().paint(self.window_id, piet, ctx)
    }

    fn size(&mut self, width: u32, height: u32, ctx: &mut dyn WinCtx) {
        let event = Event::Size(Size::new(width as f64, height as f64));
        self.do_event(event, ctx);
        //FIXME: the window needs to adjust the size based on DPI.
        //let dpi = self
        //.app_state
        //.borrow()
        //.window_state
        //.get(&self.window_id)
        //.unwrap()
        //.handle
        //.get_dpi() as f64;
        //let scale = 96.0 / dpi;
        //let event = Event::Size(Size::new(width as f64 * scale, height as f64 * scale));
        //self.do_event(event, ctx);
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

impl<T: Data> std::default::Default for Windows<T> {
    fn default() -> Self {
        Windows {
            windows: HashMap::new(),
            state: HashMap::new(),
        }
    }
}
