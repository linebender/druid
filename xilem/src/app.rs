// Copyright 2022 The Druid Authors.
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

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use druid_shell::kurbo::Size;
use druid_shell::piet::{Color, Piet, RenderContext};
use druid_shell::{IdleHandle, IdleToken, WindowHandle};
use tokio::runtime::Runtime;

use crate::event::{AsyncWake, EventResult};
use crate::id::IdPath;
use crate::widget::{CxState, EventCx, LayoutCx, PaintCx, Pod, UpdateCx, WidgetState};
use crate::{
    event::Event,
    id::Id,
    view::{Cx, View},
    widget::{RawEvent, Widget},
};

pub struct App<T, V: View<T>> {
    req_chan: tokio::sync::mpsc::Sender<AppReq>,
    response_chan: tokio::sync::mpsc::Receiver<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Sender<(V, V::State, HashSet<Id>)>,
    id: Option<Id>,
    events: Vec<Event>,
    window_handle: WindowHandle,
    root_state: WidgetState,
    root_pod: Option<Pod>,
    size: Size,
    cx: Cx,
    pub(crate) rt: Runtime,
}

/// The standard delay for waiting for async futures.
const RENDER_DELAY: Duration = Duration::from_millis(5);

/// State that's kept in a separate task for running the app
struct AppTask<T, V: View<T>, F: FnMut(&mut T) -> V> {
    req_chan: tokio::sync::mpsc::Receiver<AppReq>,
    response_chan: tokio::sync::mpsc::Sender<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Receiver<(V, V::State, HashSet<Id>)>,

    data: T,
    app_logic: F,
    view: Option<V>,
    state: Option<V::State>,
    idle_handle: Option<IdleHandle>,
    pending_async: HashSet<Id>,
    ui_state: UiState,
}

/// A message sent from the main UI thread to the app task
pub(crate) enum AppReq {
    SetIdleHandle(IdleHandle),
    Events(Vec<Event>),
    Wake(IdPath),
    // Parameter indicates whether it should be delayed for async
    Render(bool),
}

/// A response sent to a render request.
struct RenderResponse<V, S> {
    prev: Option<V>,
    view: V,
    state: Option<S>,
}

#[derive(PartialEq)]
enum UiState {
    /// Starting state, ready for events and render requests.
    Start,
    /// Received render request, haven't responded yet.
    Delayed,
    /// An async completion woke the UI thread.
    WokeUI,
}

#[derive(Clone, Default)]
pub struct WakeQueue(Arc<Mutex<Vec<IdPath>>>);

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

impl<T: Send + 'static, V: View<T> + 'static> App<T, V>
where
    V::Element: Widget + 'static,
    V::State: 'static,
{
    /// Create a new app instance.
    pub fn new(data: T, app_logic: impl FnMut(&mut T) -> V + Send + 'static) -> Self {
        // Create a new tokio runtime. Doing it here is hacky, we should allow
        // the client to do it.
        let rt = Runtime::new().unwrap();

        // Note: there is danger of deadlock if exceeded; think this through.
        const CHANNEL_SIZE: usize = 1000;
        let (req_tx, req_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(1);
        let (return_tx, return_rx) = tokio::sync::mpsc::channel(1);

        // We have a separate thread to forward wake requests (mostly generated
        // by the custom waker when we poll) to the async task. Maybe there's a
        // better way, but this is expedient.
        //
        // It's a sync_channel because sender needs to be sync to work in an async
        // context. Consider crossbeam and flume channels as alternatives.
        let req_tx_clone = req_tx.clone();
        let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(10);
        std::thread::spawn(move || {
            while let Ok(id_path) = wake_rx.recv() {
                let _ = req_tx_clone.blocking_send(AppReq::Wake(id_path));
            }
        });
        let cx = Cx::new(&wake_tx);

        // spawn app task
        rt.spawn(async move {
            let mut app_task = AppTask {
                req_chan: req_rx,
                response_chan: response_tx,
                return_chan: return_rx,
                data,
                app_logic,
                view: None,
                state: None,
                idle_handle: None,
                pending_async: HashSet::new(),
                ui_state: UiState::Start,
            };
            app_task.run().await;
        });
        App {
            req_chan: req_tx,
            response_chan: response_rx,
            return_chan: return_tx,
            id: None,
            root_pod: None,
            events: Vec::new(),
            window_handle: Default::default(),
            root_state: Default::default(),
            size: Default::default(),
            cx,
            rt,
        }
    }

    pub fn connect(&mut self, window_handle: WindowHandle) {
        self.window_handle = window_handle.clone();
        if let Some(idle_handle) = window_handle.get_idle_handle() {
            let _ = self
                .req_chan
                .blocking_send(AppReq::SetIdleHandle(idle_handle));
        }
    }

    pub fn size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn paint(&mut self, piet: &mut Piet) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);

        loop {
            self.send_events();
            self.render();
            let root_pod = self.root_pod.as_mut().unwrap();
            let mut cx_state = CxState::new(&self.window_handle, &mut self.events);
            let mut update_cx = UpdateCx::new(&mut cx_state, &mut self.root_state);
            root_pod.update(&mut update_cx);
            let mut layout_cx = LayoutCx::new(&mut cx_state, &mut self.root_state);
            root_pod.measure(&mut layout_cx);
            let proposed_size = self.size;
            root_pod.layout(&mut layout_cx, proposed_size);
            if cx_state.has_events() {
                // Rerun app logic, primarily for LayoutObserver
                // We might want some debugging here if the number of iterations
                // becomes extreme.
                continue;
            }
            let mut layout_cx = LayoutCx::new(&mut cx_state, &mut self.root_state);
            let visible = root_pod.state.size.to_rect();
            root_pod.prepare_paint(&mut layout_cx, visible);
            if cx_state.has_events() {
                // Rerun app logic, primarily for virtualized scrolling
                continue;
            }
            let mut paint_cx = PaintCx::new(&mut cx_state, &mut self.root_state, piet);
            root_pod.paint(&mut paint_cx);
            break;
        }
    }

    pub fn window_event(&mut self, event: RawEvent) {
        let root_pod = self.root_pod.as_mut().unwrap();
        let mut cx_state = CxState::new(&self.window_handle, &mut self.events);
        let mut event_cx = EventCx::new(&mut cx_state, &mut self.root_state);
        root_pod.event(&mut event_cx, &event);
        self.send_events();
    }

    fn send_events(&mut self) {
        if !self.events.is_empty() {
            let events = std::mem::take(&mut self.events);
            let _ = self.req_chan.blocking_send(AppReq::Events(events));
        }
    }

    /// Run the app logic and update the widget tree.
    fn render(&mut self) {
        if self.render_inner(false) {
            self.render_inner(true);
        }
    }

    /// Run one pass of app logic.
    ///
    /// Return value is whether there are any pending async futures.
    fn render_inner(&mut self, delay: bool) -> bool {
        self.cx.pending_async.clear();
        let _ = self.req_chan.blocking_send(AppReq::Render(delay));
        if let Some(response) = self.response_chan.blocking_recv() {
            let state =
                if let Some(element) = self.root_pod.as_mut().and_then(|pod| pod.downcast_mut()) {
                    let mut state = response.state.unwrap();
                    let changed = response.view.rebuild(
                        &mut self.cx,
                        response.prev.as_ref().unwrap(),
                        self.id.as_mut().unwrap(),
                        &mut state,
                        element,
                    );
                    if changed {
                        self.root_pod.as_mut().unwrap().request_update();
                    }
                    assert!(self.cx.is_empty(), "id path imbalance on rebuild");
                    state
                } else {
                    let (id, state, element) = response.view.build(&mut self.cx);
                    assert!(self.cx.is_empty(), "id path imbalance on build");
                    self.root_pod = Some(Pod::new(element));
                    self.id = Some(id);
                    state
                };
            let pending = std::mem::take(&mut self.cx.pending_async);
            let has_pending = !pending.is_empty();
            let _ = self
                .return_chan
                .blocking_send((response.view, state, pending));
            has_pending
        } else {
            false
        }
    }
}

impl<T, V: View<T>, F: FnMut(&mut T) -> V> AppTask<T, V, F>
where
    V::Element: Widget + 'static,
{
    async fn run(&mut self) {
        let mut deadline = None;
        loop {
            let rx = self.req_chan.recv();
            let req = match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, rx).await,
                None => Ok(rx.await),
            };
            match req {
                Ok(Some(req)) => match req {
                    AppReq::SetIdleHandle(handle) => self.idle_handle = Some(handle),
                    AppReq::Events(events) => {
                        for event in events {
                            let id_path = &event.id_path[1..];
                            self.view.as_ref().unwrap().event(
                                id_path,
                                self.state.as_mut().unwrap(),
                                event.body,
                                &mut self.data,
                            );
                        }
                    }
                    AppReq::Wake(id_path) => {
                        let result = self.view.as_ref().unwrap().event(
                            &id_path[1..],
                            self.state.as_mut().unwrap(),
                            Box::new(AsyncWake),
                            &mut self.data,
                        );
                        if matches!(result, EventResult::RequestRebuild) {
                            // request re-render from UI thread
                            if self.ui_state == UiState::Start {
                                if let Some(handle) = self.idle_handle.as_mut() {
                                    handle.schedule_idle(IdleToken::new(42));
                                }
                                self.ui_state = UiState::WokeUI;
                            }
                            let id = id_path.last().unwrap();
                            self.pending_async.remove(&id);
                            if self.pending_async.is_empty() && self.ui_state == UiState::Delayed {
                                self.render().await;
                                deadline = None;
                            }
                        }
                    }
                    AppReq::Render(delay) => {
                        if !delay || self.pending_async.is_empty() {
                            self.render().await;
                            deadline = None;
                        } else {
                            deadline = Some(tokio::time::Instant::now() + RENDER_DELAY);
                            self.ui_state = UiState::Delayed;
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    self.render().await;
                    deadline = None;
                }
            }
        }
    }

    async fn render(&mut self) {
        let view = (self.app_logic)(&mut self.data);
        let response = RenderResponse {
            prev: self.view.take(),
            view,
            state: self.state.take(),
        };
        if self.response_chan.send(response).await.is_err() {
            println!("error sending render response");
        }
        if let Some((view, state, pending)) = self.return_chan.recv().await {
            self.view = Some(view);
            self.state = Some(state);
            self.pending_async = pending;
        }
        self.ui_state = UiState::Start;
    }
}
