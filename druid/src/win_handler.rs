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
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::Instant;

use log::{error, info, warn};

use crate::kurbo::{Rect, Size, Vec2};
use crate::piet::{Piet, RenderContext};
use crate::shell::{
    Application, Cursor, FileDialogOptions, IdleToken, MouseEvent, WinCtx, WinHandler, WindowHandle,
};

use crate::app_delegate::{AppDelegate, DelegateCtx};
use crate::bloom::Bloom;
use crate::core::BaseState;
use crate::menu::ContextMenu;
use crate::theme;
use crate::window::Window;
use crate::{
    Command, Data, Env, Event, EventCtx, KeyEvent, KeyModifiers, LayoutCtx, LifeCycle,
    LifeCycleCtx, MenuDesc, PaintCtx, Target, TimerToken, UpdateCtx, WheelEvent, WindowDesc,
    WindowId,
};

use crate::command::sys as sys_cmd;

const RUN_COMMANDS_TOKEN: IdleToken = IdleToken::new(1);

/// The struct implements the druid-shell `WinHandler` trait.
///
/// One `DruidHandler` exists per window.
///
/// This is something of an internal detail and possibly we don't want to surface
/// it publicly.
pub struct DruidHandler<T: Data> {
    /// The shared app state.
    app_state: Rc<RefCell<AppState<T>>>,
    /// The id for the current window.
    window_id: WindowId,
}

/// State shared by all windows in the UI.
pub(crate) struct AppState<T: Data> {
    delegate: Option<Box<dyn AppDelegate<T>>>,
    command_queue: VecDeque<(Target, Command)>,
    windows: Windows<T>,
    pub(crate) env: Env,
    pub(crate) data: T,
}

/// All active windows.
struct Windows<T: Data> {
    windows: HashMap<WindowId, WindowEntry<T>>,
}

/// The handle and state for a window.
///
/// When we create a window, we create our internal window structure (`Window<T>`)
/// before we have access to the handle (in `WindowState`).
struct WindowEntry<T: Data> {
    id: WindowId,
    window: Option<Window<T>>,
    state: Option<WindowState>,
}

/// Per-window state not owned by user code.
pub(crate) struct WindowState {
    // TODO: should this be in `WindowEntry` by itself?
    pub(crate) handle: WindowHandle,
    prev_paint_time: Option<Instant>,
    needs_inval: bool,
}

/// Everything required for a window to handle an event.
struct SingleWindowState<'a, T: Data> {
    window_id: WindowId,
    window: &'a mut Window<T>,
    state: &'a mut WindowState,
    command_queue: &'a mut VecDeque<(Target, Command)>,
    data: &'a mut T,
    env: &'a Env,
}

impl<T: Data> WindowEntry<T> {
    fn new(id: WindowId) -> Self {
        WindowEntry {
            id,
            window: None,
            state: None,
        }
    }

    // unpacks this entry if it has both a window and state set.
    fn contents_if_some(&mut self) -> Option<(WindowId, &mut WindowState, &mut Window<T>)> {
        if let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_mut()) {
            Some((self.id, state, window))
        } else {
            None
        }
    }
}

impl<T: Data> Windows<T> {
    fn connect(&mut self, id: WindowId, handle: WindowHandle) {
        let state = WindowState {
            handle,
            prev_paint_time: None,
            needs_inval: false,
        };
        self.windows
            .entry(id)
            .or_insert_with(|| WindowEntry::new(id))
            .state = Some(state);
    }

    fn add(&mut self, id: WindowId, window: Window<T>) {
        self.windows
            .entry(id)
            .or_insert_with(|| WindowEntry::new(id))
            .window = Some(window);
    }

    fn remove(&mut self, id: WindowId) -> Option<WindowHandle> {
        self.windows
            .remove(&id)
            .and_then(|entry| entry.state)
            .map(|state| state.handle)
    }

    fn state_mut(&mut self, id: WindowId) -> Option<&mut WindowState> {
        self.get_mut(id).map(|(_, state, _)| state)
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = (WindowId, &mut WindowState, &mut Window<T>)> {
        self.windows
            .values_mut()
            .flat_map(WindowEntry::contents_if_some)
    }

    fn get_menu_cmd(&self, window_id: WindowId, cmd_id: u32) -> Option<Command> {
        self.windows
            .get(&window_id)
            .and_then(|entry| entry.window.as_ref())
            .and_then(|w| w.get_menu_cmd(cmd_id))
    }

    fn get_mut(&mut self, id: WindowId) -> Option<(WindowId, &mut WindowState, &mut Window<T>)> {
        self.windows
            .get_mut(&id)
            .and_then(WindowEntry::contents_if_some)
    }
}

impl<'a, T: Data> SingleWindowState<'a, T> {
    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        let request_anim = self.do_anim_frame(ctx);
        self.do_layout(piet);
        piet.clear(self.env.get(theme::WINDOW_BACKGROUND_COLOR));
        self.do_paint(piet);

        // schedule an idle call with the runloop if there are commands to process after painting is finished,
        // that would trigger a new event/update pass.
        if !self.command_queue.is_empty() {
            if let Some(mut handle) = self.state.handle.get_idle_handle() {
                handle.schedule_idle(RUN_COMMANDS_TOKEN);
            } else {
                error!("failed to get idle handle");
            }
        }

        request_anim
    }

    fn do_anim_frame(&mut self, ctx: &mut dyn WinCtx) -> bool {
        // TODO: this calculation uses wall-clock time of the paint call, which
        // potentially has jitter.
        //
        // See https://github.com/xi-editor/druid/issues/85 for discussion.
        let this_paint_time = Instant::now();
        let prev_paint_time = self.state.prev_paint_time;
        let interval = if let Some(last) = prev_paint_time {
            let duration = this_paint_time.duration_since(last);
            1_000_000_000 * duration.as_secs() + u64::from(duration.subsec_nanos())
        } else {
            0
        };
        let anim_frame_event = Event::AnimFrame(interval);
        let (_, request_anim) = self.do_event_inner(anim_frame_event, ctx);
        let prev = if request_anim {
            Some(this_paint_time)
        } else {
            None
        };
        self.state.prev_paint_time = prev;
        request_anim
    }

    fn do_layout(&mut self, piet: &mut Piet) {
        let mut layout_ctx = LayoutCtx {
            text_factory: piet.text(),
            window_id: self.window_id,
        };
        self.window.layout(&mut layout_ctx, self.data, self.env);
    }

    fn do_paint(&mut self, piet: &mut Piet) {
        let base_state = BaseState::default();
        let mut paint_ctx = PaintCtx {
            render_ctx: piet,
            base_state: &base_state,
            window_id: self.window_id,
            region: Rect::ZERO.into(),
        };
        self.window.paint(&mut paint_ctx, self.data, self.env);
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns two flags. The first is true if the event was handled. The
    /// second is true if an animation frame is requested.
    fn do_event_inner(&mut self, event: Event, win_ctx: &mut dyn WinCtx) -> (bool, bool) {
        // should there be a root base state persisting in the ui state instead?
        let mut cursor = match event {
            Event::MouseMoved(..) => Some(Cursor::Arrow),
            _ => None,
        };

        let event = match event {
            Event::Size(size) => {
                let dpi = f64::from(self.state.handle.get_dpi());
                let scale = 96.0 / dpi;
                Event::Size(Size::new(size.width * scale, size.height * scale))
            }
            other => other,
        };

        let mut base_state = BaseState::default();
        let mut ctx = EventCtx {
            win_ctx,
            cursor: &mut cursor,
            command_queue: self.command_queue,
            base_state: &mut base_state,
            is_handled: false,
            is_root: true,
            had_active: self.window.root.has_active(),
            window: &self.state.handle,
            window_id: self.window_id,
            widget_id: self.window.root.id(),
        };
        self.window.event(&mut ctx, &event, self.data, self.env);

        let is_handled = ctx.is_handled;
        if ctx.base_state.request_focus {
            let focus_event = Event::FocusChanged(true);
            self.window
                .event(&mut ctx, &focus_event, self.data, self.env);
        }
        self.state.needs_inval = ctx.base_state.needs_inval | ctx.base_state.request_anim;
        let request_anim = ctx.base_state.request_anim;
        if let Some(cursor) = cursor {
            win_ctx.set_cursor(&cursor);
        }

        (is_handled, request_anim)
    }

    fn do_lifecycle(&mut self, event: LifeCycle, _win_ctx: &mut dyn WinCtx) {
        let mut ctx = LifeCycleCtx {
            command_queue: self.command_queue,
            children: Bloom::default(),
            window_id: self.window_id,
            widget_id: self.window.root.id(),
        };
        self.window.lifecycle(&mut ctx, &event, self.data, self.env);
    }

    fn set_menu(&mut self, cmd: &Command) {
        let mut menu = match cmd.get_object::<MenuDesc<T>>() {
            Some(menu) => menu.to_owned(),
            None => {
                warn!("set-menu command is missing menu object");
                return;
            }
        };

        let platform_menu = menu.build_window_menu(&self.data, &self.env);
        self.state.handle.set_menu(platform_menu);
        self.window.menu = Some(menu);
    }

    fn show_context_menu(&mut self, cmd: &Command) {
        let (mut menu, point) = match cmd.get_object::<ContextMenu<T>>() {
            Some(ContextMenu { menu, location }) => (menu.to_owned(), *location),
            None => {
                warn!("show-context-menu command is missing menu object.");
                return;
            }
        };
        let platform_menu = menu.build_popup_menu(&self.data, &self.env);
        self.state.handle.show_context_menu(platform_menu, point);
        self.window.context_menu = Some(menu);
    }

    fn window_got_focus(&mut self) {
        #[cfg(target_os = "macos")]
        self.macos_update_app_menu()
    }

    /// On macos we need to update the global application menu to be the menu
    /// for the current window.
    #[cfg(target_os = "macos")]
    fn macos_update_app_menu(&mut self) {
        let SingleWindowState {
            window,
            state,
            data,
            env,
            ..
        } = self;
        let platform_menu = window
            .menu
            .as_mut()
            .map(|m| m.build_window_menu(&data, &env));
        if let Some(platform_menu) = platform_menu {
            state.handle.set_menu(platform_menu);
        }
    }
}

impl<T: Data> AppState<T> {
    pub(crate) fn new(
        data: T,
        env: Env,
        delegate: Option<Box<dyn AppDelegate<T>>>,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(AppState {
            delegate,
            command_queue: VecDeque::new(),
            data,
            env,
            windows: Windows::default(),
        }))
    }

    fn get_menu_cmd(&self, window_id: WindowId, cmd_id: u32) -> Option<Command> {
        self.windows.get_menu_cmd(window_id, cmd_id)
    }

    /// A helper fn for setting up the `DelegateCtx`. Takes a closure with
    /// an arbitrary return type `R`, and returns `Some(R)` if an `AppDelegate`
    /// is configured.
    fn with_delegate<R, F>(&mut self, id: WindowId, f: F) -> Option<R>
    where
        F: FnOnce(&mut Box<dyn AppDelegate<T>>, &mut T, &Env, &mut DelegateCtx) -> R,
    {
        let AppState {
            ref mut delegate,
            ref mut command_queue,
            ref mut data,
            ref env,
            ..
        } = self;
        let mut ctx = DelegateCtx {
            source_id: id,
            command_queue,
        };
        if let Some(delegate) = delegate {
            Some(f(delegate, data, env, &mut ctx))
        } else {
            None
        }
    }

    fn delegate_event(&mut self, id: WindowId, event: Event) -> Option<Event> {
        if self.delegate.is_some() {
            self.with_delegate(id, |del, data, env, ctx| del.event(event, data, env, ctx))
                .unwrap()
        } else {
            Some(event)
        }
    }

    fn connect(&mut self, id: WindowId, handle: WindowHandle) {
        self.windows.connect(id, handle);
        self.with_delegate(id, |del, data, env, ctx| {
            del.window_added(id, data, env, ctx)
        });
    }

    pub(crate) fn add_window(&mut self, id: WindowId, window: Window<T>) {
        self.windows.add(id, window);
    }

    /// Called after this window has been closed by the platform.
    ///
    /// We clean up resources and notifiy the delegate, if necessary.
    fn remove_window(&mut self, window_id: WindowId, _ctx: &mut dyn WinCtx) {
        self.with_delegate(window_id, |del, data, env, ctx| {
            del.window_removed(window_id, data, env, ctx)
        });
        self.windows.remove(window_id);
    }

    /// triggered by a menu item or other command.
    ///
    /// This doesn't close the window; it calls the close method on the platform
    /// window handle; the platform should close the window, and then call
    /// our handlers `destroy()` method, at which point we can do our cleanup.
    fn request_close_window(&mut self, window_id: WindowId) {
        if let Some(state) = self.windows.state_mut(window_id) {
            state.handle.close();
        }
    }

    fn show_window(&mut self, id: WindowId) {
        if let Some(state) = self.windows.state_mut(id) {
            state.handle.bring_to_front_and_focus();
        }
    }

    fn assemble_window_state(&mut self, window_id: WindowId) -> Option<SingleWindowState<'_, T>> {
        let AppState {
            ref mut command_queue,
            ref mut windows,
            ref mut data,
            ref env,
            ..
        } = self;
        windows
            .get_mut(window_id)
            .map(move |(window_id, state, window)| SingleWindowState {
                window_id,
                window,
                state,
                command_queue,
                data,
                env,
            })
    }

    fn paint(&mut self, window_id: WindowId, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        self.assemble_window_state(window_id)
            .map(|mut win| win.paint(piet, ctx))
            .unwrap_or(false)
    }

    fn do_event(&mut self, source_id: WindowId, event: Event, win_ctx: &mut dyn WinCtx) -> bool {
        // if the event was swallowed by the delegate we consider it handled?
        let event = match self.delegate_event(source_id, event) {
            Some(event) => event,
            None => return true,
        };

        if let Event::TargetedCommand(_target, ref cmd) = event {
            match cmd.selector {
                sys_cmd::SET_MENU => {
                    if let Some(mut win) = self.assemble_window_state(source_id) {
                        win.set_menu(cmd);
                    }
                    return true;
                }
                sys_cmd::SHOW_CONTEXT_MENU => {
                    if let Some(mut win) = self.assemble_window_state(source_id) {
                        win.show_context_menu(cmd);
                    }
                    return true;
                }
                _ => (),
            }
        }

        let is_handled = match event {
            Event::TargetedCommand(Target::Widget(_), _) => {
                let mut any_handled = false;

                // TODO: this is using the WinCtx of the window originating the event,
                // rather than a WinCtx appropriate to the target window. This probably
                // needs to get rethought.
                for (window_id, state, window) in self.windows.iter_mut() {
                    let mut win = SingleWindowState {
                        window_id,
                        command_queue: &mut self.command_queue,
                        state,
                        window,
                        data: &mut self.data,
                        env: &self.env,
                    };
                    let (handled, _) = win.do_event_inner(event.clone(), win_ctx);
                    any_handled |= handled;
                    if handled {
                        break;
                    }
                }
                any_handled
            }
            _ => {
                let (handled, _) = self
                    .assemble_window_state(source_id)
                    .map(|mut win| win.do_event_inner(event, win_ctx))
                    .unwrap_or((false, false));
                handled
            }
        };

        //TODO: should we be always calling update after an event? shouldn't
        //we at least handle commands first?
        self.do_update(win_ctx);

        is_handled
    }

    fn do_update(&mut self, win_ctx: &mut dyn WinCtx) {
        // we send `update` to all windows, not just the active one:
        for (id, state, window) in self.windows.iter_mut() {
            let mut update_ctx = UpdateCtx {
                text_factory: win_ctx.text_factory(),
                window: &state.handle,
                command_queue: &mut self.command_queue,
                needs_inval: false,
                window_id: id,
                widget_id: window.root.id(),
            };
            window.update(&mut update_ctx, &self.data, &self.env);
            state.needs_inval |= update_ctx.needs_inval;
        }
        self.invalidate_if_needed();
    }

    /// invalidate any window handles that need it.
    ///
    /// This should always be called at the end of an event update cycle,
    /// including for lifecycle events.
    fn invalidate_if_needed(&mut self) {
        for (_, state, _) in self.windows.iter_mut() {
            if state.needs_inval {
                state.handle.invalidate();
                state.needs_inval = false;
            }
        }
    }

    fn window_got_focus(&mut self, window_id: WindowId, _ctx: &mut dyn WinCtx) {
        self.assemble_window_state(window_id)
            .as_mut()
            .map(SingleWindowState::window_got_focus);
    }
}

impl<T: Data> DruidHandler<T> {
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
        let result = self
            .app_state
            .borrow_mut()
            .do_event(self.window_id, event, win_ctx);
        self.process_commands(win_ctx);
        result
    }

    // Is this only for events with an external origin, like AnimFrame?
    // other lifecycle events can happen from mid-tree. What are our guarantees?
    fn do_lifecycle(&mut self, event: LifeCycle, win_ctx: &mut dyn WinCtx) {
        self.app_state
            .borrow_mut()
            .assemble_window_state(self.window_id)
            .map(|mut win| win.do_lifecycle(event, win_ctx));
        //FIXME: maybe we don't want to do commands now, but rather schedule idle?
        self.process_commands(win_ctx);
    }

    fn process_commands(&mut self, win_ctx: &mut dyn WinCtx) {
        loop {
            let next_cmd = self.app_state.borrow_mut().command_queue.pop_front();
            match next_cmd {
                Some((target, cmd)) => self.handle_cmd(target, cmd, win_ctx),
                None => break,
            }
        }
    }

    fn handle_system_cmd(&mut self, cmd_id: u32, win_ctx: &mut dyn WinCtx) {
        let cmd = self.app_state.borrow().get_menu_cmd(self.window_id, cmd_id);
        match cmd {
            Some(cmd) => self
                .app_state
                .borrow_mut()
                .command_queue
                .push_back((self.window_id.into(), cmd)),
            None => warn!("No command for menu id {}", cmd_id),
        }
        self.process_commands(win_ctx)
    }

    /// Handle a command. Top level commands (e.g. for creating and destroying windows)
    /// have their logic here; other commands are passed to the window.
    fn handle_cmd(&mut self, target: Target, cmd: Command, win_ctx: &mut dyn WinCtx) {
        //FIXME: we need some way of getting the correct `WinCtx` for this window.
        if let Target::Window(window_id) = target {
            match &cmd.selector {
                &sys_cmd::SHOW_OPEN_PANEL => self.show_open_panel(cmd, window_id, win_ctx),
                &sys_cmd::SHOW_SAVE_PANEL => self.show_save_panel(cmd, window_id, win_ctx),
                &sys_cmd::NEW_WINDOW => self.new_window(cmd),
                &sys_cmd::CLOSE_WINDOW => self.request_close_window(cmd, window_id),
                &sys_cmd::SHOW_WINDOW => self.show_window(cmd),
                &sys_cmd::QUIT_APP => self.quit(),
                &sys_cmd::HIDE_APPLICATION => self.hide_app(),
                &sys_cmd::HIDE_OTHERS => self.hide_others(),
                &sys_cmd::PASTE => self.do_paste(window_id, win_ctx),
                sel => {
                    info!("handle_cmd {}", sel);
                    let event = Event::TargetedCommand(target, cmd);
                    self.app_state
                        .borrow_mut()
                        .do_event(window_id, event, win_ctx);
                }
            }
        } else {
            info!("handle_cmd {} -> widget", cmd.selector);
            let event = Event::TargetedCommand(target, cmd);
            // TODO: self.window_id the correct source identifier here?
            self.app_state
                .borrow_mut()
                .do_event(self.window_id, event, win_ctx);
        }
    }

    fn show_open_panel(&mut self, cmd: Command, window_id: WindowId, win_ctx: &mut dyn WinCtx) {
        let options = cmd
            .get_object::<FileDialogOptions>()
            .map(|opts| opts.to_owned())
            .unwrap_or_default();
        let result = win_ctx.open_file_sync(options);
        if let Some(info) = result {
            let cmd = Command::new(sys_cmd::OPEN_FILE, info);
            let event = Event::TargetedCommand(window_id.into(), cmd);
            self.app_state
                .borrow_mut()
                .do_event(window_id, event, win_ctx);
        }
    }

    fn show_save_panel(&mut self, cmd: Command, window_id: WindowId, win_ctx: &mut dyn WinCtx) {
        let options = cmd
            .get_object::<FileDialogOptions>()
            .map(|opts| opts.to_owned())
            .unwrap_or_default();
        let result = win_ctx.save_as_sync(options);
        if let Some(info) = result {
            let cmd = Command::new(sys_cmd::SAVE_FILE, info);
            let event = Event::TargetedCommand(window_id.into(), cmd);
            self.app_state
                .borrow_mut()
                .do_event(window_id, event, win_ctx);
        }
    }

    fn new_window(&mut self, cmd: Command) {
        let desc = match cmd.get_object::<WindowDesc<T>>() {
            Some(wd) => wd,
            None => {
                warn!("new_window command is missing window description");
                return;
            }
        };

        let window = match desc.build_native(&self.app_state) {
            Ok(win) => win,
            Err(e) => {
                error!("failed to create window: '{:?}'", e);
                return;
            }
        };
        window.show();
    }

    fn request_close_window(&mut self, cmd: Command, window_id: WindowId) {
        let id = cmd.get_object().unwrap_or(&window_id);
        self.app_state.borrow_mut().request_close_window(*id);
    }

    fn show_window(&mut self, cmd: Command) {
        let id: WindowId = *cmd
            .get_object()
            .expect("show window selector missing window id");
        self.app_state.borrow_mut().show_window(id);
    }

    fn do_paste(&mut self, window_id: WindowId, ctx: &mut dyn WinCtx) {
        let event = Event::Paste(Application::clipboard());
        self.app_state.borrow_mut().do_event(window_id, event, ctx);
    }

    fn quit(&self) {
        Application::quit()
    }

    fn hide_app(&self) {
        #[cfg(all(target_os = "macos", not(feature = "use_gtk")))]
        Application::hide()
    }

    fn hide_others(&mut self) {
        #[cfg(all(target_os = "macos", not(feature = "use_gtk")))]
        Application::hide_others()
    }
}

impl<T: Data> WinHandler for DruidHandler<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        //NOTE: this method predates `connected`, and we call delegate methods here.
        //it's possible that we should move those calls to occur in connected?
        self.app_state
            .borrow_mut()
            .connect(self.window_id, handle.clone());
    }

    fn connected(&mut self, ctx: &mut dyn WinCtx) {
        self.do_lifecycle(LifeCycle::WidgetAdded, ctx);
        self.do_lifecycle(LifeCycle::RegisterChildren, ctx);
        self.do_lifecycle(LifeCycle::WindowConnected, ctx);
    }

    fn paint(&mut self, piet: &mut Piet, ctx: &mut dyn WinCtx) -> bool {
        self.app_state.borrow_mut().paint(self.window_id, piet, ctx)
    }

    fn size(&mut self, width: u32, height: u32, ctx: &mut dyn WinCtx) {
        let event = Event::Size(Size::new(f64::from(width), f64::from(height)));
        self.do_event(event, ctx);
    }

    fn command(&mut self, id: u32, ctx: &mut dyn WinCtx) {
        self.handle_system_cmd(id, ctx);
    }

    fn mouse_down(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        // TODO: double-click detection (or is this done in druid-shell?)
        let event = Event::MouseDown(event.clone().into());
        self.do_event(event, ctx);
    }

    fn mouse_up(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        let event = Event::MouseUp(event.clone().into());
        self.do_event(event, ctx);
    }

    fn mouse_move(&mut self, event: &MouseEvent, ctx: &mut dyn WinCtx) {
        let event = Event::MouseMoved(event.clone().into());
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

    fn zoom(&mut self, delta: f64, ctx: &mut dyn WinCtx) {
        let event = Event::Zoom(delta);
        self.do_event(event, ctx);
    }

    fn got_focus(&mut self, ctx: &mut dyn WinCtx) {
        self.app_state
            .borrow_mut()
            .window_got_focus(self.window_id, ctx);
    }

    fn timer(&mut self, token: TimerToken, ctx: &mut dyn WinCtx) {
        self.do_event(Event::Timer(token), ctx);
    }

    fn idle(&mut self, token: IdleToken, ctx: &mut dyn WinCtx) {
        if token == RUN_COMMANDS_TOKEN {
            self.process_commands(ctx);
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn destroy(&mut self, ctx: &mut dyn WinCtx) {
        self.app_state
            .borrow_mut()
            .remove_window(self.window_id, ctx);
    }
}

impl<T: Data> Default for Windows<T> {
    fn default() -> Self {
        Windows {
            windows: HashMap::new(),
        }
    }
}
