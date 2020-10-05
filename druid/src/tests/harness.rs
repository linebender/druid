// Copyright 2020 The Druid Authors.
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

use std::path::Path;
use std::sync::Arc;

use crate::app::PendingWindow;
use crate::core::{CommandQueue, WidgetState};
use crate::ext_event::ExtEventHost;
use crate::piet::{BitmapTarget, Device, Error, ImageFormat, Piet};
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
pub struct Harness<'a, T> {
    piet: Piet<'a>,
    inner: Inner<T>,
    window_size: Size,
}

/// All of the state except for the `Piet` (render context). We need to pass
/// that in to get around some lifetime issues.
struct Inner<T> {
    data: T,
    env: Env,
    window: Window<T>,
    cmds: CommandQueue,
}

/// A way to clean up resources when our target goes out of scope.
// the inner type is an option so that we can take ownership in `drop` even
// though self is `& mut`.
pub struct TargetGuard<'a>(Option<BitmapTarget<'a>>);

impl<'a> TargetGuard<'a> {
    /// Turns the TargetGuard into a array of pixels
    #[allow(dead_code)]
    pub fn into_raw(mut self) -> Arc<[u8]> {
        let mut raw_target = self.0.take().unwrap();
        raw_target
            .to_image_buf(ImageFormat::RgbaPremul)
            .unwrap()
            .raw_pixels_shared()
    }

    /// Saves the TargetGuard into a png
    #[allow(dead_code)]
    pub fn into_png<P: AsRef<Path>>(mut self, path: P) -> Result<(), Error> {
        let raw_target = self.0.take().unwrap();
        raw_target.save_to_file(path)
    }
}

impl<T: Data> Harness<'_, T> {
    /// Create a new `Harness` with the given data and a root widget,
    /// and provide that harness to the passed in function.
    ///
    /// For lifetime reasons™, we cannot just make a harness. It's complicated.
    /// I tried my best.
    ///
    /// This function is a subset of [create_with_render](struct.Harness.html#create_with_render)
    pub fn create_simple(
        data: T,
        root: impl Widget<T> + 'static,
        harness_closure: impl FnMut(&mut Harness<T>),
    ) {
        Self::create_with_render(data, root, DEFAULT_SIZE, harness_closure, |_target| {})
    }

    /// Create a new `Harness` with the given data and a root widget,
    /// and provide that harness to the `harness_closure` callback and then the
    /// render_context to the `render_context_closure` callback.
    ///
    /// For lifetime reasons™, we cannot just make a harness. It's complicated.
    /// I tried my best.
    ///
    /// The with_render version of `create` also has a callback that can be used
    /// to save or inspect the painted widget
    ///
    /// # Usage
    ///
    /// The create functions are used to test a widget. The function takes a `root` widget
    /// and a data structure and uses them to create a `Harness`. The Harness can then be interacted
    /// with via the `harness_closure` callback. The the final render of
    /// the widget can be inspected with the `render_context_closure` callback.
    ///
    /// # Arguments
    ///
    /// * `data` - A structure that matches the type of the widget and that will be
    ///  passed to the `harness_closure` callback via the `Harness` structure.
    ///
    /// * `root` - The widget under test
    ///
    /// * `shape` - The shape of the render_context in the `Harness` structure
    ///
    /// * `harness_closure` - A closure used to interact with the widget under test through the
    /// `Harness` structure.
    ///
    /// * `render_context_closure` - A closure used to inspect the final render_context via the `TargetGuard` structure.
    ///
    pub fn create_with_render(
        data: T,
        root: impl Widget<T> + 'static,
        window_size: Size,
        mut harness_closure: impl FnMut(&mut Harness<T>),
        mut render_context_closure: impl FnMut(TargetGuard),
    ) {
        let ext_host = ExtEventHost::default();
        let ext_handle = ext_host.make_sink();
        let mut device = Device::new().expect("harness failed to get device");
        let target = device
            .bitmap_target(window_size.width as usize, window_size.height as usize, 1.0)
            .expect("bitmap_target");
        let mut target = TargetGuard(Some(target));
        {
            let piet = target.0.as_mut().unwrap().render_context();

            let pending = PendingWindow::new(|| root);
            let window = Window::new(WindowId::next(), Default::default(), pending, ext_handle);

            let inner = Inner {
                data,
                env: Env::default(),
                window,
                cmds: Default::default(),
            };

            let mut harness = Harness {
                piet,
                inner,
                window_size,
            };
            harness_closure(&mut harness);
        }
        render_context_closure(target)
    }

    /// Set the size without sending a resize event; intended to be used
    /// before calling `send_initial_events`
    pub fn set_initial_size(&mut self, size: Size) {
        self.window_size = size;
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

    /// Retrieve a copy of this widget's `WidgetState`, or die trying.
    pub(crate) fn get_state(&mut self, widget: WidgetId) -> WidgetState {
        match self.try_get_state(widget) {
            Some(thing) => thing,
            None => panic!("get_state failed for widget {:?}", widget),
        }
    }

    /// Attempt to retrieve a copy of this widget's `WidgetState`.
    pub(crate) fn try_get_state(&mut self, widget: WidgetId) -> Option<WidgetState> {
        let cell = StateCell::default();
        let state_cell = cell.clone();
        self.lifecycle(LifeCycle::Internal(InternalLifeCycle::DebugRequestState {
            widget,
            state_cell,
        }));
        cell.take()
    }

    /// Inspect the `WidgetState` of each widget in the tree.
    ///
    /// The provided closure will be called on each widget.
    pub(crate) fn inspect_state(&mut self, f: impl Fn(&WidgetState) + 'static) {
        let checkfn = StateCheckFn::new(f);
        self.lifecycle(LifeCycle::Internal(InternalLifeCycle::DebugInspectState(
            checkfn,
        )))
    }

    /// Send a command to a target.
    pub fn submit_command(&mut self, cmd: impl Into<Command>) {
        let command = cmd.into().default_to(self.inner.window.id.into());
        let event = Event::Internal(InternalEvent::TargetedCommand(command));
        self.event(event);
    }

    /// Send the events that would normally be sent when the app starts.
    // should we do this automatically? Also these will change regularly?
    pub fn send_initial_events(&mut self) {
        self.event(Event::WindowConnected);
        self.event(Event::WindowSize(self.window_size));
    }

    /// Send an event to the widget.
    ///
    /// If this event triggers lifecycle events, they will also be dispatched,
    /// as will any resulting commands. This will also trigger `update`.
    ///
    /// Commands dispatched during `update` will not be sent?
    pub fn event(&mut self, event: Event) {
        self.inner.event(event);
        self.process_commands();
        self.update();
    }

    fn process_commands(&mut self) {
        loop {
            let cmd = self.inner.cmds.pop_front();
            match cmd {
                Some(cmd) => self.event(Event::Internal(InternalEvent::TargetedCommand(cmd))),
                None => break,
            }
        }
    }

    fn lifecycle(&mut self, event: LifeCycle) {
        self.inner.lifecycle(event)
    }

    //TODO: should we expose this? I don't think so?
    fn update(&mut self) {
        self.inner.update()
    }

    /// Only do a layout pass, without painting
    pub fn just_layout(&mut self) {
        self.inner.layout()
    }

    /// Paints just the part of the window that was invalidated by calls to `request_paint` or
    /// `request_paint_rect`.
    ///
    /// Also resets the invalid region.
    #[allow(dead_code)]
    pub fn paint_invalid(&mut self) {
        let invalid = std::mem::replace(self.window_mut().invalid_mut(), Region::EMPTY);
        self.inner.paint_region(&mut self.piet, &invalid);
    }

    /// Paints the entire window and resets the invalid region.
    #[allow(dead_code)]
    pub fn paint(&mut self) {
        self.window_mut().invalid_mut().clear();
        self.inner
            .paint_region(&mut self.piet, &self.window_size.to_rect().into());
    }
}

impl<T: Data> Inner<T> {
    fn event(&mut self, event: Event) {
        self.window
            .event(&mut self.cmds, event, &mut self.data, &self.env);
    }

    fn lifecycle(&mut self, event: LifeCycle) {
        self.window
            .lifecycle(&mut self.cmds, &event, &self.data, &self.env, false);
    }

    fn update(&mut self) {
        self.window.update(&mut self.cmds, &self.data, &self.env);
    }

    fn layout(&mut self) {
        self.window
            .just_layout(&mut self.cmds, &self.data, &self.env);
    }

    #[allow(dead_code)]
    fn paint_region(&mut self, piet: &mut Piet, invalid: &Region) {
        self.window
            .do_paint(piet, &invalid, &mut self.cmds, &self.data, &self.env);
    }
}

impl<T> Drop for Harness<'_, T> {
    fn drop(&mut self) {
        // We need to call finish even if a test assert failed
        if let Err(err) = self.piet.finish() {
            // We can't panic, because we might already be panicking
            log::error!("piet finish failed: {}", err);
        }
    }
}

impl Drop for TargetGuard<'_> {
    fn drop(&mut self) {
        // we need to call this to clean up the context
        let _ = self
            .0
            .take()
            .map(|mut t| t.to_image_buf(piet::ImageFormat::RgbaPremul));
    }
}
