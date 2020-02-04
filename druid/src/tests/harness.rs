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
use crate::piet::{BitmapTarget, Device, Piet};
use crate::window::PendingWindow;
use crate::*;

pub(crate) const DEFAULT_SIZE: Size = Size::new(400., 400.);

/// A type that tries very hard to provide a comforting and safe environment
/// for widgets who are trying to find their way.
///
/// You create a `Harness` with some widget and its initial data; then you
/// can send events to that widget and verify that expected conditions are met.
///
/// Harness tries to act like the normal druid environment; for instance, it will
/// attempt to dispatch any `Command`s that are sent during event handling, and
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
pub struct Harness<'a, T: Data> {
    piet: Piet<'a>,
    inner: Inner<T>,
}

/// All of the state except for the `Piet` (render context). We need to pass
/// that in to get around some lifetime issues.
struct Inner<T: Data> {
    data: T,
    env: Env,
    window: Window<T>,
    cmds: CommandQueue,
}

/// A `WinCtx` impl that we can conjure from the ether.
pub struct MockWinCtx<'a, 't: 'a>(&'a mut Text<'t>);

/// A way to clean up resources when our target goes out of scope.
// the inner type is an option so that we can take ownership in `drop` even
// though self is `& mut`.
struct TargetGuard<'a>(Option<BitmapTarget<'a>>);

impl<T: Data> Harness<'_, T> {
    /// Create a new `Harness` with the given data and a root widget,
    /// and provide that harness to the passed in function.
    ///
    /// For lifetime reasons™, we cannot just make a harness. It's complicated.
    /// I tried my best.
    pub fn create(data: T, root: impl Widget<T> + 'static, mut f: impl FnMut(&mut Harness<T>)) {
        let mut device = Device::new().expect("harness failed to get device");
        let target = device.bitmap_target(400, 400, 2.).expect("bitmap_target");
        let mut target = TargetGuard(Some(target));
        let piet = target.0.as_mut().unwrap().render_context();

        let inner = Inner {
            data,
            env: theme::init(),
            window: PendingWindow::new(root, LocalizedString::new(""), None)
                .into_window(WindowId::next(), Default::default()),
            cmds: Default::default(),
        };

        let mut harness = Harness { piet, inner };
        f(&mut harness);
    }

    pub fn window(&self) -> &Window<T> {
        &self.inner.window
    }

    #[allow(dead_code)]
    pub fn window_mut(&mut self) -> &mut Window<T> {
        &mut self.inner.window
    }

    #[allow(dead_code)]
    pub fn data(&self) -> &T {
        &self.inner.data
    }

    /// Retrieve a copy of this widget's `BaseState`, if possible.
    //FIXME: make this unwrap, and add a `try_get_state` variant?
    pub(crate) fn get_state(&mut self, widget: WidgetId) -> Option<BaseState> {
        let cell = StateCell::default();
        let state_cell = cell.clone();
        self.lifecycle(LifeCycle::DebugRequestState { widget, state_cell });
        cell.take()
    }

    /// Inspect the `BaseState` of each widget in the tree.
    ///
    /// The provided closure will be called on each widget.
    pub(crate) fn inspect_state(&mut self, f: impl Fn(&BaseState) + 'static) {
        let checkfn = StateCheckFn::new(f);
        self.lifecycle(LifeCycle::DebugInspectState(checkfn))
    }

    /// Send a command to a target.
    pub fn submit_command(&mut self, cmd: impl Into<Command>, target: impl Into<Option<Target>>) {
        let target = target.into().unwrap_or_else(|| self.inner.window.id.into());
        let event = Event::TargetedCommand(target, cmd.into());
        self.event(event);
    }

    /// Send the events that would normally be sent when the app starts.
    // should we do this automatically? Also these will change regularly?
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
        self.inner.event(event, &mut self.piet);
        self.process_commands();
        self.update();
    }

    fn process_commands(&mut self) {
        loop {
            let cmd = self.inner.cmds.pop_front();
            match cmd {
                Some((target, cmd)) => self.event(Event::TargetedCommand(target, cmd)),
                None => break,
            }
        }
    }

    #[allow(dead_code)]
    pub fn lifecycle(&mut self, event: LifeCycle) {
        self.inner.lifecycle(event)
    }

    //TODO: should we expose this? I don't think so?
    fn update(&mut self) {
        self.inner.update(&mut self.piet)
    }

    /// Only do a layout pass, without painting
    pub fn just_layout(&mut self) {
        self.inner.layout(&mut self.piet)
    }

    #[allow(dead_code)]
    pub fn paint(&mut self) {
        self.inner.paint(&mut self.piet)
    }
}

impl<T: Data> Inner<T> {
    fn event(&mut self, event: Event, piet: &mut Piet) {
        let mut win_ctx = MockWinCtx(piet.text());
        self.window.event(
            &mut win_ctx,
            &mut self.cmds,
            event,
            &mut self.data,
            &self.env,
        );
    }

    #[allow(dead_code)]
    fn lifecycle(&mut self, event: LifeCycle) {
        self.window
            .lifecycle(&mut self.cmds, &event, &self.data, &self.env);
    }

    fn update(&mut self, piet: &mut Piet) {
        let mut win_ctx = MockWinCtx(piet.text());
        self.window.update(&mut win_ctx, &self.data, &self.env);
    }

    fn layout(&mut self, piet: &mut Piet) {
        self.window.just_layout(piet, &self.data, &self.env);
    }

    #[allow(dead_code)]
    fn paint(&mut self, piet: &mut Piet) {
        self.window
            .do_paint(piet, &mut self.cmds, &self.data, &self.env);
    }
}

impl<'a, 't> WinCtx<'t> for MockWinCtx<'a, 't> {
    fn invalidate(&mut self) {}
    fn text_factory(&mut self) -> &mut Text<'t> {
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

impl Drop for TargetGuard<'_> {
    fn drop(&mut self) {
        // we need to call this to clean up the context
        let _ = self
            .0
            .take()
            .map(|t| t.into_raw_pixels(piet::ImageFormat::RgbaPremul));
    }
}
