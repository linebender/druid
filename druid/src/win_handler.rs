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

//! The implementation of the WinHandler trait (druid-shell integration).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::kurbo::Size;
use crate::piet::Piet;
use crate::shell::{
    Application, FileDialogToken, FileInfo, IdleToken, MouseEvent, Region, Scale, WinHandler,
    WindowHandle,
};

use crate::app_delegate::{AppDelegate, DelegateCtx};
use crate::core::CommandQueue;
use crate::ext_event::{ExtEventHost, ExtEventSink};
use crate::menu::ContextMenu;
use crate::window::Window;
use crate::{
    Command, Data, Env, Event, Handled, InternalEvent, KeyEvent, MenuDesc, PlatformError, Target,
    TimerToken, WindowDesc, WindowId,
};

use crate::app::{PendingWindow, WindowConfig};
use crate::command::sys as sys_cmd;
use druid_shell::WindowBuilder;

pub(crate) const RUN_COMMANDS_TOKEN: IdleToken = IdleToken::new(1);

/// A token we are called back with if an external event was submitted.
pub(crate) const EXT_EVENT_IDLE_TOKEN: IdleToken = IdleToken::new(2);

/// The struct implements the druid-shell `WinHandler` trait.
///
/// One `DruidHandler` exists per window.
///
/// This is something of an internal detail and possibly we don't want to surface
/// it publicly.
pub struct DruidHandler<T> {
    /// The shared app state.
    app_state: AppState<T>,
    /// The id for the current window.
    window_id: WindowId,
}

/// The top level event handler.
///
/// This corresponds to the `AppHandler` trait in druid-shell, which is only
/// used to handle events that are not associated with a window.
///
/// Currently, this means only menu items on macOS when no window is open.
pub(crate) struct AppHandler<T> {
    app_state: AppState<T>,
}

/// State shared by all windows in the UI.
#[derive(Clone)]
pub(crate) struct AppState<T> {
    inner: Rc<RefCell<Inner<T>>>,
}

struct Inner<T> {
    app: Application,
    delegate: Option<Box<dyn AppDelegate<T>>>,
    command_queue: CommandQueue,
    file_dialogs: HashMap<FileDialogToken, WindowId>,
    ext_event_host: ExtEventHost,
    windows: Windows<T>,
    /// the application-level menu, only set on macos and only if there
    /// are no open windows.
    root_menu: Option<MenuDesc<T>>,
    pub(crate) env: Env,
    pub(crate) data: T,
}

/// All active windows.
struct Windows<T> {
    pending: HashMap<WindowId, PendingWindow<T>>,
    windows: HashMap<WindowId, Window<T>>,
}

impl<T> Windows<T> {
    fn connect(&mut self, id: WindowId, handle: WindowHandle, ext_handle: ExtEventSink) {
        if let Some(pending) = self.pending.remove(&id) {
            let win = Window::new(id, handle, pending, ext_handle);
            assert!(self.windows.insert(id, win).is_none(), "duplicate window");
        } else {
            log::error!("no window for connecting handle {:?}", id);
        }
    }

    fn add(&mut self, id: WindowId, win: PendingWindow<T>) {
        assert!(self.pending.insert(id, win).is_none(), "duplicate pending");
    }

    fn remove(&mut self, id: WindowId) -> Option<Window<T>> {
        self.windows.remove(&id)
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut Window<T>> {
        self.windows.values_mut()
    }

    fn get(&self, id: WindowId) -> Option<&Window<T>> {
        self.windows.get(&id)
    }

    fn get_mut(&mut self, id: WindowId) -> Option<&mut Window<T>> {
        self.windows.get_mut(&id)
    }

    fn count(&self) -> usize {
        self.windows.len() + self.pending.len()
    }
}

impl<T> AppHandler<T> {
    pub(crate) fn new(app_state: AppState<T>) -> Self {
        Self { app_state }
    }
}

impl<T> AppState<T> {
    pub(crate) fn new(
        app: Application,
        data: T,
        env: Env,
        delegate: Option<Box<dyn AppDelegate<T>>>,
        ext_event_host: ExtEventHost,
    ) -> Self {
        let inner = Rc::new(RefCell::new(Inner {
            app,
            delegate,
            command_queue: VecDeque::new(),
            file_dialogs: HashMap::new(),
            root_menu: None,
            ext_event_host,
            data,
            env,
            windows: Windows::default(),
        }));

        AppState { inner }
    }

    pub(crate) fn app(&self) -> Application {
        self.inner.borrow().app.clone()
    }
}

impl<T: Data> Inner<T> {
    fn get_menu_cmd(&self, window_id: Option<WindowId>, cmd_id: u32) -> Option<Command> {
        match window_id {
            Some(id) => self.windows.get(id).and_then(|w| w.get_menu_cmd(cmd_id)),
            None => self
                .root_menu
                .as_ref()
                .and_then(|m| m.command_for_id(cmd_id)),
        }
    }

    fn append_command(&mut self, cmd: Command) {
        self.command_queue.push_back(cmd);
    }

    /// A helper fn for setting up the `DelegateCtx`. Takes a closure with
    /// an arbitrary return type `R`, and returns `Some(R)` if an `AppDelegate`
    /// is configured.
    fn with_delegate<R, F>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Box<dyn AppDelegate<T>>, &mut T, &Env, &mut DelegateCtx) -> R,
    {
        let Inner {
            ref mut delegate,
            ref mut command_queue,
            ref mut data,
            ref env,
            ..
        } = self;
        let mut ctx = DelegateCtx {
            command_queue,
            app_data_type: TypeId::of::<T>(),
        };
        if let Some(delegate) = delegate {
            Some(f(delegate, data, env, &mut ctx))
        } else {
            None
        }
    }

    fn delegate_event(&mut self, id: WindowId, event: Event) -> Option<Event> {
        if self.delegate.is_some() {
            self.with_delegate(|del, data, env, ctx| del.event(ctx, id, event, data, env))
                .unwrap()
        } else {
            Some(event)
        }
    }

    fn delegate_cmd(&mut self, cmd: &Command) -> Handled {
        self.with_delegate(|del, data, env, ctx| del.command(ctx, cmd.target(), cmd, data, env))
            .unwrap_or(Handled::No)
    }

    fn connect(&mut self, id: WindowId, handle: WindowHandle) {
        self.windows
            .connect(id, handle, self.ext_event_host.make_sink());

        // If the external event host has no handle, it cannot wake us
        // when an event arrives.
        if self.ext_event_host.handle_window_id.is_none() {
            self.set_ext_event_idle_handler(id);
        }

        self.with_delegate(|del, data, env, ctx| del.window_added(id, data, env, ctx));
    }

    /// Called after this window has been closed by the platform.
    ///
    /// We clean up resources and notifiy the delegate, if necessary.
    fn remove_window(&mut self, window_id: WindowId) {
        self.with_delegate(|del, data, env, ctx| del.window_removed(window_id, data, env, ctx));
        // when closing the last window:
        if let Some(mut win) = self.windows.remove(window_id) {
            if self.windows.windows.is_empty() {
                // on mac we need to keep the menu around
                self.root_menu = win.menu.take();
                // If there are even no pending windows, we quit the run loop.
                if self.windows.count() == 0 {
                    #[cfg(any(target_os = "windows", feature = "x11"))]
                    self.app.quit();
                }
            }
        }

        // if we are closing the window that is currently responsible for
        // waking us when external events arrive, we want to pass that responsibility
        // to another window.
        if self.ext_event_host.handle_window_id == Some(window_id) {
            self.ext_event_host.handle_window_id = None;
            // find any other live window
            let win_id = self.windows.windows.keys().find(|k| *k != &window_id);
            if let Some(any_other_window) = win_id.cloned() {
                self.set_ext_event_idle_handler(any_other_window);
            }
        }
    }

    /// Set the idle handle that will be used to wake us when external events arrive.
    fn set_ext_event_idle_handler(&mut self, id: WindowId) {
        if let Some(mut idle) = self
            .windows
            .get_mut(id)
            .and_then(|win| win.handle.get_idle_handle())
        {
            if self.ext_event_host.has_pending_items() {
                idle.schedule_idle(EXT_EVENT_IDLE_TOKEN);
            }
            self.ext_event_host.set_idle(idle, id);
        }
    }

    /// triggered by a menu item or other command.
    ///
    /// This doesn't close the window; it calls the close method on the platform
    /// window handle; the platform should close the window, and then call
    /// our handlers `destroy()` method, at which point we can do our cleanup.
    fn request_close_window(&mut self, window_id: WindowId) {
        if let Some(win) = self.windows.get_mut(window_id) {
            win.handle.close();
        }
    }

    /// Requests the platform to close all windows.
    fn request_close_all_windows(&mut self) {
        for win in self.windows.iter_mut() {
            win.handle.close();
        }
    }

    fn show_window(&mut self, id: WindowId) {
        if let Some(win) = self.windows.get_mut(id) {
            win.handle.bring_to_front_and_focus();
        }
    }

    fn configure_window(&mut self, config: &WindowConfig, id: WindowId) {
        if let Some(win) = self.windows.get_mut(id) {
            config.apply_to_handle(&mut win.handle);
        }
    }

    fn prepare_paint(&mut self, window_id: WindowId) {
        if let Some(win) = self.windows.get_mut(window_id) {
            win.prepare_paint(&mut self.command_queue, &mut self.data, &self.env);
        }
        self.do_update();
    }

    fn paint(&mut self, window_id: WindowId, piet: &mut Piet, invalid: &Region) {
        if let Some(win) = self.windows.get_mut(window_id) {
            win.do_paint(
                piet,
                invalid,
                &mut self.command_queue,
                &self.data,
                &self.env,
            );
        }
    }

    fn dispatch_cmd(&mut self, cmd: Command) -> Handled {
        let handled = self.delegate_cmd(&cmd);
        self.do_update();
        if handled.is_handled() {
            return handled;
        }

        match cmd.target() {
            Target::Window(id) => {
                // first handle special window-level events
                if cmd.is(sys_cmd::SET_MENU) {
                    self.set_menu(id, &cmd);
                    return Handled::Yes;
                }
                if cmd.is(sys_cmd::SHOW_CONTEXT_MENU) {
                    self.show_context_menu(id, &cmd);
                    return Handled::Yes;
                }
                if let Some(w) = self.windows.get_mut(id) {
                    let event = Event::Command(cmd);
                    return w.event(&mut self.command_queue, event, &mut self.data, &self.env);
                }
            }
            // in this case we send it to every window that might contain
            // this widget, breaking if the event is handled.
            Target::Widget(id) => {
                for w in self.windows.iter_mut().filter(|w| w.may_contain_widget(id)) {
                    let event = Event::Internal(InternalEvent::TargetedCommand(cmd.clone()));
                    if w.event(&mut self.command_queue, event, &mut self.data, &self.env)
                        .is_handled()
                    {
                        return Handled::Yes;
                    }
                }
            }
            Target::Global => {
                for w in self.windows.iter_mut() {
                    let event = Event::Command(cmd.clone());
                    if w.event(&mut self.command_queue, event, &mut self.data, &self.env)
                        .is_handled()
                    {
                        return Handled::Yes;
                    }
                }
            }
            Target::Auto => {
                log::error!("{:?} reached window handler with `Target::Auto`", cmd);
            }
        }
        Handled::No
    }

    fn do_window_event(&mut self, source_id: WindowId, event: Event) -> Handled {
        match event {
            Event::Command(..) | Event::Internal(InternalEvent::TargetedCommand(..)) => {
                panic!("commands should be dispatched via dispatch_cmd");
            }
            _ => (),
        }

        // if the event was swallowed by the delegate we consider it handled?
        let event = match self.delegate_event(source_id, event) {
            Some(event) => event,
            None => return Handled::Yes,
        };

        if let Some(win) = self.windows.get_mut(source_id) {
            win.event(&mut self.command_queue, event, &mut self.data, &self.env)
        } else {
            Handled::No
        }
    }

    fn set_menu(&mut self, window_id: WindowId, cmd: &Command) {
        if let Some(win) = self.windows.get_mut(window_id) {
            match cmd
                .get_unchecked(sys_cmd::SET_MENU)
                .downcast_ref::<MenuDesc<T>>()
            {
                Some(menu) => win.set_menu(menu.clone(), &self.data, &self.env),
                None => panic!(
                    "{} command must carry a MenuDesc<application state>.",
                    sys_cmd::SET_MENU
                ),
            }
        }
    }

    fn show_context_menu(&mut self, window_id: WindowId, cmd: &Command) {
        if let Some(win) = self.windows.get_mut(window_id) {
            match cmd
                .get_unchecked(sys_cmd::SHOW_CONTEXT_MENU)
                .downcast_ref::<ContextMenu<T>>()
            {
                Some(ContextMenu { menu, location }) => {
                    win.show_context_menu(menu.to_owned(), *location, &self.data, &self.env)
                }
                None => panic!(
                    "{} command must carry a ContextMenu<application state>.",
                    sys_cmd::SHOW_CONTEXT_MENU
                ),
            }
        }
    }

    fn do_update(&mut self) {
        // we send `update` to all windows, not just the active one:
        for window in self.windows.iter_mut() {
            window.update(&mut self.command_queue, &self.data, &self.env);
        }
        self.invalidate_and_finalize();
    }

    /// invalidate any window handles that need it.
    ///
    /// This should always be called at the end of an event update cycle,
    /// including for lifecycle events.
    fn invalidate_and_finalize(&mut self) {
        for win in self.windows.iter_mut() {
            win.invalidate_and_finalize();
        }
    }

    #[cfg(target_os = "macos")]
    fn window_got_focus(&mut self, window_id: WindowId) {
        if let Some(win) = self.windows.get_mut(window_id) {
            win.macos_update_app_menu(&self.data, &self.env)
        }
    }
    #[cfg(not(target_os = "macos"))]
    fn window_got_focus(&mut self, _: WindowId) {}
}

impl<T: Data> DruidHandler<T> {
    /// Note: the root widget doesn't go in here, because it gets added to the
    /// app state.
    pub(crate) fn new_shared(app_state: AppState<T>, window_id: WindowId) -> DruidHandler<T> {
        DruidHandler {
            app_state,
            window_id,
        }
    }
}

impl<T: Data> AppState<T> {
    pub(crate) fn data(&self) -> T {
        self.inner.borrow().data.clone()
    }

    pub(crate) fn env(&self) -> Env {
        self.inner.borrow().env.clone()
    }

    pub(crate) fn add_window(&self, id: WindowId, window: PendingWindow<T>) {
        self.inner.borrow_mut().windows.add(id, window);
    }

    fn connect_window(&mut self, window_id: WindowId, handle: WindowHandle) {
        self.inner.borrow_mut().connect(window_id, handle)
    }

    fn remove_window(&mut self, window_id: WindowId) {
        self.inner.borrow_mut().remove_window(window_id)
    }

    fn window_got_focus(&mut self, window_id: WindowId) {
        self.inner.borrow_mut().window_got_focus(window_id)
    }

    /// Send an event to the widget hierarchy.
    ///
    /// Returns `true` if the event produced an action.
    ///
    /// This is principally because in certain cases (such as keydown on Windows)
    /// the OS needs to know if an event was handled.
    fn do_window_event(&mut self, event: Event, window_id: WindowId) -> Handled {
        let result = self.inner.borrow_mut().do_window_event(window_id, event);
        self.process_commands();
        self.inner.borrow_mut().do_update();
        result
    }

    fn prepare_paint_window(&mut self, window_id: WindowId) {
        self.inner.borrow_mut().prepare_paint(window_id);
    }

    fn paint_window(&mut self, window_id: WindowId, piet: &mut Piet, invalid: &Region) {
        self.inner.borrow_mut().paint(window_id, piet, invalid);
    }

    fn idle(&mut self, token: IdleToken) {
        match token {
            RUN_COMMANDS_TOKEN => {
                self.process_commands();
                self.inner.borrow_mut().do_update();
            }
            EXT_EVENT_IDLE_TOKEN => {
                self.process_ext_events();
                self.process_commands();
                self.inner.borrow_mut().do_update();
            }
            other => log::warn!("unexpected idle token {:?}", other),
        }
    }

    fn process_commands(&mut self) {
        loop {
            let next_cmd = self.inner.borrow_mut().command_queue.pop_front();
            match next_cmd {
                Some(cmd) => self.handle_cmd(cmd),
                None => break,
            }
        }
    }

    fn process_ext_events(&mut self) {
        loop {
            let ext_cmd = self.inner.borrow_mut().ext_event_host.recv();
            match ext_cmd {
                Some(cmd) => self.handle_cmd(cmd),
                None => break,
            }
        }
    }

    /// Handle a 'command' message from druid-shell. These map to  an item
    /// in an application, window, or context (right-click) menu.
    ///
    /// If the menu is  associated with a window (the general case) then
    /// the `window_id` will be `Some(_)`, otherwise (such as if no window
    /// is open but a menu exists, as on macOS) it will be `None`.
    fn handle_system_cmd(&mut self, cmd_id: u32, window_id: Option<WindowId>) {
        let cmd = self.inner.borrow().get_menu_cmd(window_id, cmd_id);
        match cmd {
            Some(cmd) => {
                let default_target = window_id.map(Into::into).unwrap_or(Target::Global);
                self.inner
                    .borrow_mut()
                    .append_command(cmd.default_to(default_target))
            }
            None => log::warn!("No command for menu id {}", cmd_id),
        }
        self.process_commands();
        self.inner.borrow_mut().do_update();
    }

    /// Handle a command. Top level commands (e.g. for creating and destroying
    /// windows) have their logic here; other commands are passed to the window.
    fn handle_cmd(&mut self, cmd: Command) {
        use Target as T;
        match cmd.target() {
            // these are handled the same no matter where they come from
            _ if cmd.is(sys_cmd::QUIT_APP) => self.quit(),
            _ if cmd.is(sys_cmd::HIDE_APPLICATION) => self.hide_app(),
            _ if cmd.is(sys_cmd::HIDE_OTHERS) => self.hide_others(),
            _ if cmd.is(sys_cmd::NEW_WINDOW) => {
                if let Err(e) = self.new_window(cmd) {
                    log::error!("failed to create window: '{}'", e);
                }
            }
            _ if cmd.is(sys_cmd::CLOSE_ALL_WINDOWS) => self.request_close_all_windows(),
            // these should come from a window
            // FIXME: we need to be able to open a file without a window handle
            T::Window(id) if cmd.is(sys_cmd::SHOW_OPEN_PANEL) => self.show_open_panel(cmd, id),
            T::Window(id) if cmd.is(sys_cmd::SHOW_SAVE_PANEL) => self.show_save_panel(cmd, id),
            T::Window(id) if cmd.is(sys_cmd::CONFIGURE_WINDOW) => self.configure_window(cmd, id),
            T::Window(id) if cmd.is(sys_cmd::CLOSE_WINDOW) => {
                if !self.inner.borrow_mut().dispatch_cmd(cmd).is_handled() {
                    self.request_close_window(id);
                }
            }
            T::Window(id) if cmd.is(sys_cmd::SHOW_WINDOW) => self.show_window(id),
            T::Window(id) if cmd.is(sys_cmd::PASTE) => self.do_paste(id),
            _ if cmd.is(sys_cmd::CLOSE_WINDOW) => {
                log::warn!("CLOSE_WINDOW command must target a window.")
            }
            _ if cmd.is(sys_cmd::SHOW_WINDOW) => {
                log::warn!("SHOW_WINDOW command must target a window.")
            }
            _ => {
                self.inner.borrow_mut().dispatch_cmd(cmd);
            }
        }
    }

    fn show_open_panel(&mut self, cmd: Command, window_id: WindowId) {
        let options = cmd.get_unchecked(sys_cmd::SHOW_OPEN_PANEL).to_owned();
        let handle = self
            .inner
            .borrow_mut()
            .windows
            .get_mut(window_id)
            .map(|w| w.handle.clone());

        let token = handle.and_then(|mut handle| handle.open_file(options));
        if let Some(token) = token {
            self.inner
                .borrow_mut()
                .file_dialogs
                .insert(token, window_id);
        }
    }

    fn show_save_panel(&mut self, cmd: Command, window_id: WindowId) {
        let options = cmd.get_unchecked(sys_cmd::SHOW_SAVE_PANEL).to_owned();
        let handle = self
            .inner
            .borrow_mut()
            .windows
            .get_mut(window_id)
            .map(|w| w.handle.clone());

        let token = handle.and_then(|mut handle| handle.save_as(options));
        if let Some(token) = token {
            self.inner
                .borrow_mut()
                .file_dialogs
                .insert(token, window_id);
        }
    }

    fn new_window(&mut self, cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
        let desc = cmd.get_unchecked(sys_cmd::NEW_WINDOW);
        // The NEW_WINDOW command is private and only druid can receive it by normal means,
        // thus unwrapping can be considered safe and deserves a panic.
        let desc = desc.take().unwrap().downcast::<WindowDesc<T>>().unwrap();
        let window = desc.build_native(self)?;
        window.show();
        Ok(())
    }

    fn request_close_window(&mut self, id: WindowId) {
        self.inner.borrow_mut().request_close_window(id);
    }

    fn request_close_all_windows(&mut self) {
        self.inner.borrow_mut().request_close_all_windows();
    }

    fn show_window(&mut self, id: WindowId) {
        self.inner.borrow_mut().show_window(id);
    }

    fn configure_window(&mut self, cmd: Command, id: WindowId) {
        if let Some(config) = cmd.get(sys_cmd::CONFIGURE_WINDOW) {
            self.inner.borrow_mut().configure_window(config, id);
        }
    }

    fn do_paste(&mut self, window_id: WindowId) {
        let event = Event::Paste(self.inner.borrow().app.clipboard());
        self.inner.borrow_mut().do_window_event(window_id, event);
    }

    fn quit(&self) {
        self.inner.borrow().app.quit()
    }

    fn hide_app(&self) {
        #[cfg(target_os = "macos")]
        self.inner.borrow().app.hide()
    }

    fn hide_others(&mut self) {
        #[cfg(target_os = "macos")]
        self.inner.borrow().app.hide_others()
    }

    pub(crate) fn build_native_window(
        &mut self,
        id: WindowId,
        mut pending: PendingWindow<T>,
        config: WindowConfig,
    ) -> Result<WindowHandle, PlatformError> {
        let mut builder = WindowBuilder::new(self.app());
        config.apply_to_builder(&mut builder);

        let data = self.data();
        let env = self.env();

        pending.title.resolve(&data, &env);
        builder.set_title(pending.title.display_text().to_string());

        let platform_menu = pending
            .menu
            .as_mut()
            .map(|m| m.build_window_menu(&data, &env));
        if let Some(menu) = platform_menu {
            builder.set_menu(menu);
        }

        let handler = DruidHandler::new_shared((*self).clone(), id);
        builder.set_handler(Box::new(handler));

        self.add_window(id, pending);
        builder.build()
    }
}

impl<T: Data> crate::shell::AppHandler for AppHandler<T> {
    fn command(&mut self, id: u32) {
        self.app_state.handle_system_cmd(id, None)
    }
}

impl<T: Data> WinHandler for DruidHandler<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.app_state
            .connect_window(self.window_id, handle.clone());

        let event = Event::WindowConnected;
        self.app_state.do_window_event(event, self.window_id);
    }

    fn prepare_paint(&mut self) {
        self.app_state.prepare_paint_window(self.window_id);
    }

    fn paint(&mut self, piet: &mut Piet, region: &Region) {
        self.app_state.paint_window(self.window_id, piet, region);
    }

    fn size(&mut self, size: Size) {
        let event = Event::WindowSize(size);
        self.app_state.do_window_event(event, self.window_id);
    }

    fn scale(&mut self, _scale: Scale) {
        // TODO: Do something with the scale
    }

    fn command(&mut self, id: u32) {
        self.app_state.handle_system_cmd(id, Some(self.window_id));
    }

    fn save_as(&mut self, token: FileDialogToken, file_info: Option<FileInfo>) {
        let mut inner = self.app_state.inner.borrow_mut();
        if let Some(window_id) = inner.file_dialogs.remove(&token) {
            let cmd = if let Some(info) = file_info {
                sys_cmd::SAVE_FILE.with(Some(info)).to(window_id)
            } else {
                sys_cmd::SAVE_PANEL_CANCELLED.to(window_id)
            };
            inner.dispatch_cmd(cmd);
        } else {
            log::error!("unknown save dialog token");
        }
    }

    fn open_file(&mut self, token: FileDialogToken, file_info: Option<FileInfo>) {
        let mut inner = self.app_state.inner.borrow_mut();
        if let Some(window_id) = inner.file_dialogs.remove(&token) {
            let cmd = if let Some(info) = file_info {
                sys_cmd::OPEN_FILE.with(info).to(window_id)
            } else {
                sys_cmd::OPEN_PANEL_CANCELLED.to(window_id)
            };
            inner.dispatch_cmd(cmd);
        } else {
            log::error!("unknown open dialog token");
        }
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        // TODO: double-click detection (or is this done in druid-shell?)
        let event = Event::MouseDown(event.clone().into());
        self.app_state.do_window_event(event, self.window_id);
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        let event = Event::MouseUp(event.clone().into());
        self.app_state.do_window_event(event, self.window_id);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        let event = Event::MouseMove(event.clone().into());
        self.app_state.do_window_event(event, self.window_id);
    }

    fn mouse_leave(&mut self) {
        self.app_state
            .do_window_event(Event::Internal(InternalEvent::MouseLeave), self.window_id);
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        self.app_state
            .do_window_event(Event::KeyDown(event), self.window_id)
            .is_handled()
    }

    fn key_up(&mut self, event: KeyEvent) {
        self.app_state
            .do_window_event(Event::KeyUp(event), self.window_id);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        self.app_state
            .do_window_event(Event::Wheel(event.clone().into()), self.window_id);
    }

    fn zoom(&mut self, delta: f64) {
        let event = Event::Zoom(delta);
        self.app_state.do_window_event(event, self.window_id);
    }

    fn got_focus(&mut self) {
        self.app_state.window_got_focus(self.window_id);
    }

    fn timer(&mut self, token: TimerToken) {
        self.app_state
            .do_window_event(Event::Timer(token), self.window_id);
    }

    fn idle(&mut self, token: IdleToken) {
        self.app_state.idle(token);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn request_close(&mut self) {
        self.app_state
            .handle_cmd(sys_cmd::CLOSE_WINDOW.to(self.window_id));
        self.app_state.inner.borrow_mut().do_update();
    }

    fn destroy(&mut self) {
        self.app_state.remove_window(self.window_id);
    }
}

impl<T> Default for Windows<T> {
    fn default() -> Self {
        Windows {
            windows: HashMap::new(),
            pending: HashMap::new(),
        }
    }
}
