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

use druid_shell::{kurbo::Point, piet::Piet};

use crate::{
    event::Event,
    id::{Id, IdPath},
    view::View,
    widget::{RawEvent, Widget},
};

pub struct App<T, V: View<T, ()>, F: FnMut(&mut T) -> V>
where
    V::Element: Widget,
{
    data: T,
    app_logic: F,
    view: V,
    id: Id,
    state: V::State,
    element: V::Element,
    events: Vec<Event>,
    id_path: IdPath,
}

impl<T, V: View<T, ()>, F: FnMut(&mut T) -> V> App<T, V, F>
where
    V::Element: Widget,
{
    pub fn new(mut data: T, mut app_logic: F) -> Self {
        let mut id_path = IdPath::default();
        let view = (app_logic)(&mut data);
        let (id, state, element) = view.build(&mut id_path);
        assert!(id_path.is_empty(), "id path imbalance");
        App {
            data,
            app_logic,
            view,
            id,
            state,
            element,
            events: Vec::new(),
            id_path,
        }
    }

    pub fn paint(&mut self, piet: &mut Piet) {
        self.element.layout();
        self.element.paint(piet, Point::ZERO);
    }

    pub fn mouse_down(&mut self, point: Point) {
        self.event(RawEvent::MouseDown(point));
    }

    fn event(&mut self, event: RawEvent) {
        self.element.event(&event, &mut self.events);
        self.run_app_logic();
    }

    pub fn run_app_logic(&mut self) {
        for event in self.events.drain(..) {
            let id_path = &event.id_path[1..];
            self.view
                .event(id_path, &self.state, event.body, &mut self.data);
        }
        let view = (self.app_logic)(&mut self.data);
        view.rebuild(
            &mut self.id_path,
            &self.view,
            &mut self.id,
            &mut self.state,
            &mut self.element,
        );
        self.view = view;
    }
}
