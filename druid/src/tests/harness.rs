// Copyright 2020 The xi-editor Authors.
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

//! Tools and infrastructure for testing widgets.

use crate::core::{BaseState, CommandQueue};
use crate::piet::Device;
use crate::*;

const DEFAULT_SIZE: Size = Size::new(400., 400.);

/// A type that tries very hard to provide a comforting and safe environment
/// for widgets who are trying to find their way.
///
/// You create a `Harness` with some widget and its initial data; then you
/// can send events to that widget and verify that expected conditions are met.
///
/// Harness tries to act like the normal druid environment; for instance, it will
/// attempt to dispatch and `Command`s that are sent during event handling, and
/// it will call `update` automatically after an event.
///
/// That said, it _is_ missing a bunch of logic that would normally be handled
/// in `AppState`: for instance it does not clear the `needs_inval` and
/// `children_changed` flags on the window after an update.
///
/// In addition, layout and paint **are not called automatically**. This is
/// because paint is triggered by druid-shell, and there is no druid-shell here;
///
/// if you want those functions run you will need to call them yourself.
///
/// Also, timers don't work.  ¯\_(ツ)_/¯
pub struct Harness<T: Data> {
    device: Device,
    pub data: T,
    pub env: Env,
    pub window: Window<T>,
    handle: WindowHandle,
    command_queue: CommandQueue,
    cursor: Option<Cursor>,
    window_id: WindowId,
}

/// A `WinCtx` impl that we can conjure from the ether.
pub struct MockWinCtx<'a>(&'a mut Text<'a>);

impl<T: Data> Harness<T> {
    /// Create a new `Harness` with the given data and a root widget,
    /// and provide that harness to the passed in function.
    ///
    /// For lifetime reasons™, we cannot just make a harness. It's complicated.
    /// I tried my best.
    pub fn create(data: T, root: impl Widget<T> + 'static, mut f: impl FnMut(&mut Harness<T>)) {
        let device = Device::new().expect("harness failed to get device");
        let mut mocks = Harness {
            device,
            data,
            env: theme::init(),
            window: Window::new(root, LocalizedString::new(""), None),
            handle: Default::default(),
            command_queue: Default::default(),
            cursor: Default::default(),
            window_id: WindowId::next(),
        };

        f(&mut mocks);
    }

    /// Retrieve a copy of this widget's `BaseState`, if possible.
    pub(crate) fn get_state(&mut self, widget: WidgetId) -> Option<BaseState> {
        let cell = StateCell::default();
        let state_cell = cell.clone();
        self.lifecycle(LifeCycle::DebugRequestState { widget, state_cell });
        cell.take()
    }

    /// Send the events that would normally be sent when the app starts.
    // should we do this automatically? Also these will change regularly?
    #[allow(dead_code)]
    pub fn send_initial_events(&mut self) {
        self.lifecycle(LifeCycle::WidgetAdded);
        self.lifecycle(LifeCycle::Register);
        self.lifecycle(LifeCycle::WindowConnected);
        self.event(Event::Size(DEFAULT_SIZE));
    }

    /// Send an event to the widget.
    ///
    /// If this event triggers lifecycle events, they will also be dispatched,
    /// as will any resulting commands. This will also trigger `update`.
    ///
    /// Commands dispatched during `update` will not be sent?
    pub fn event(&mut self, event: Event) {
        let mut base_state = BaseState::new(self.window.root.id());

        // we need to instantiate this stuff in each method in order to get
        // around lifetime issues.
        //
        // we could fix this by having two types, a 'harness' and a 'harness host';
        // the latter would house just a render context and the harness,
        // and would pass the render context into the harness on each call.
        let mut target = self
            .device
            .bitmap_target(400, 400, 100.)
            .expect("harness failed to create target");
        let mut piet = target.render_context();
        let text = piet.text();
        let mut win_ctx = MockWinCtx(text);

        let mut ctx = EventCtx {
            win_ctx: &mut win_ctx,
            cursor: &mut self.cursor,
            command_queue: &mut self.command_queue,
            window_id: self.window_id,
            window: &self.handle,
            base_state: &mut base_state,
            focus_widget: None,
            had_active: false,
            is_handled: false,
            is_root: true,
        };

        self.window
            .event(&mut ctx, &event, &mut self.data, &self.env);
        self.process_commands();
        self.update();
    }

    fn process_commands(&mut self) {
        loop {
            let cmd = self.command_queue.pop_front();
            match cmd {
                Some((target, cmd)) => self.event(Event::TargetedCommand(target, cmd)),
                None => break,
            }
        }
    }

    #[allow(dead_code)]
    pub fn lifecycle(&mut self, event: LifeCycle) {
        let mut ctx = LifeCycleCtx {
            command_queue: &mut self.command_queue,
            children: Default::default(),
            window_id: self.window_id,
            widget_id: self.window.root.id(),
            focus_widgets: Vec::new(),
            request_anim: false,
            needs_inval: false,
            children_changed: false,
        };

        self.window
            .lifecycle(&mut ctx, &event, &self.data, &self.env);
    }

    //TODO: should we expose this? I don't think so?
    fn update(&mut self) {
        let mut target = self
            .device
            .bitmap_target(400, 400, 100.)
            .expect("harness failed to create target");
        let mut piet = target.render_context();

        let mut ctx = UpdateCtx {
            text_factory: piet.text(),
            window: &self.handle,
            needs_inval: false,
            children_changed: false,
            window_id: self.window_id,
            widget_id: self.window.root.id(),
        };
        self.window.update(&mut ctx, &self.data, &self.env);
    }

    #[allow(dead_code)]
    pub fn layout(&mut self) {
        let mut target = self
            .device
            .bitmap_target(400, 400, 100.)
            .expect("harness failed to create target");
        let mut piet = target.render_context();

        let mut ctx = LayoutCtx {
            text_factory: piet.text(),
            window_id: self.window_id,
        };
        self.window.layout(&mut ctx, &self.data, &self.env);
    }

    #[allow(dead_code)]
    pub fn paint(&mut self) {
        let base_state = BaseState::new(self.window.root.id());
        let mut target = self
            .device
            .bitmap_target(400, 400, 100.)
            .expect("harness failed to create target");
        let mut piet = target.render_context();

        let mut ctx = PaintCtx {
            render_ctx: &mut piet,
            window_id: self.window_id,
            region: Rect::ZERO.into(),
            base_state: &base_state,
            focus_widget: self.window.focus,
        };
        self.window.paint(&mut ctx, &self.data, &self.env);
    }
}

impl<'a> WinCtx<'a> for MockWinCtx<'a> {
    fn invalidate(&mut self) {}
    fn text_factory(&mut self) -> &mut Text<'a> {
        self.0
    }

    fn set_cursor(&mut self, _cursor: &Cursor) {}
    //TODO: we could actually implement timers if we were ambitious
    fn request_timer(&mut self, _deadline: std::time::Instant) -> TimerToken {
        TimerToken::next()
    }
    fn open_file_sync(&mut self, _: FileDialogOptions) -> Option<FileInfo> {
        None
    }
    fn save_as_sync(&mut self, _: FileDialogOptions) -> Option<FileInfo> {
        None
    }
}
