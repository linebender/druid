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

use druid_shell::kurbo::Size;
use druid_shell::piet::{Color, Piet, RenderContext};
use druid_shell::WindowHandle;

use crate::widget::{CxState, EventCx, LayoutCx, PaintCx, Pod, UpdateCx, WidgetState};
use crate::{
    event::Event,
    id::Id,
    view::{Cx, View},
    widget::{RawEvent, Widget},
};

pub struct App<T, V: View<T>, F: FnMut(&mut T) -> V> {
    data: T,
    app_logic: F,
    view: Option<V>,
    id: Option<Id>,
    state: Option<V::State>,
    events: Vec<Event>,
    window_handle: WindowHandle,
    root_state: WidgetState,
    root_pod: Option<Pod>,
    size: Size,
    cx: Cx,
}

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

impl<T, V: View<T>, F: FnMut(&mut T) -> V> App<T, V, F>
where
    V::Element: Widget + 'static,
{
    pub fn new(data: T, app_logic: F) -> Self {
        let cx = Cx::new();
        App {
            data,
            app_logic,
            view: None,
            id: None,
            state: None,
            root_pod: None,
            events: Vec::new(),
            window_handle: Default::default(),
            root_state: Default::default(),
            size: Default::default(),
            cx,
        }
    }

    pub fn ensure_app(&mut self) {
        if self.view.is_none() {
            let view = (self.app_logic)(&mut self.data);
            let (id, state, element) = view.build(&mut self.cx);
            let root_pod = Pod::new(element);
            self.view = Some(view);
            self.id = Some(id);
            self.state = Some(state);
            self.root_pod = Some(root_pod);
        }
    }

    pub fn connect(&mut self, window_handle: WindowHandle) {
        self.window_handle = window_handle.clone();
        // This will be needed for wiring up async but is a stub for now.
        //self.cx.set_handle(window_handle.get_idle_handle());
    }

    pub fn size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn paint(&mut self, piet: &mut Piet) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);

        self.ensure_app();
        loop {
            let root_pod = self.root_pod.as_mut().unwrap();
            let mut cx_state = CxState::new(&self.window_handle, &mut self.events);
            let mut update_cx = UpdateCx::new(&mut cx_state, &mut self.root_state);
            root_pod.update(&mut update_cx);
            let mut layout_cx = LayoutCx::new(&mut cx_state, &mut self.root_state);
            root_pod.measure(&mut layout_cx);
            let proposed_size = self.size;
            root_pod.layout(&mut layout_cx, proposed_size);
            if cx_state.has_events() {
                // We might want some debugging here if the number of iterations
                // becomes extreme.
                self.run_app_logic();
            } else {
                let mut paint_cx = PaintCx::new(&mut cx_state, &mut self.root_state, piet);
                root_pod.paint(&mut paint_cx);
                break;
            }
        }
    }

    pub fn window_event(&mut self, event: RawEvent) {
        self.ensure_app();
        let root_pod = self.root_pod.as_mut().unwrap();
        let mut cx_state = CxState::new(&self.window_handle, &mut self.events);
        let mut event_cx = EventCx::new(&mut cx_state, &mut self.root_state);
        root_pod.event(&mut event_cx, &event);
        self.run_app_logic();
    }

    pub fn run_app_logic(&mut self) {
        for event in self.events.drain(..) {
            let id_path = &event.id_path[1..];
            self.view.as_ref().unwrap().event(
                id_path,
                self.state.as_mut().unwrap(),
                event.body,
                &mut self.data,
            );
        }
        // Re-rendering should be more lazy.
        let view = (self.app_logic)(&mut self.data);
        if let Some(element) = self.root_pod.as_mut().unwrap().downcast_mut() {
            let changed = view.rebuild(
                &mut self.cx,
                self.view.as_ref().unwrap(),
                self.id.as_mut().unwrap(),
                self.state.as_mut().unwrap(),
                element,
            );
            if changed {
                self.root_pod.as_mut().unwrap().request_update();
            }
            assert!(self.cx.is_empty(), "id path imbalance on rebuild");
        }
        self.view = Some(view);
    }
}
