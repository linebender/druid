// Copyright 2021 The Druid Authors.
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

//! A container for a window.

use druid_shell::kurbo::Point;

use crate::element::{Action, Column, Element, Event};
use crate::tree::{Id, Mutation};

pub struct Window {
    app_logic: Box<dyn FnMut(Vec<Action>) -> Mutation>,
    // Note: given we have a root id here, it's possible we should
    // just require an id for every node in `TreeStructure`.
    root_id: Id,
    root: Column,
    actions: Vec<Action>,
}

impl Window {
    pub fn new(app_logic: Box<dyn FnMut(Vec<Action>) -> Mutation>) -> Window {
        Window {
            app_logic,
            root_id: Id::next(),
            root: Default::default(),
            actions: Vec::new(),
        }
    }

    pub fn paint(&mut self, piet: &mut druid_shell::piet::Piet) {
        self.root.layout();
        self.root.paint(piet, Point::new(0.0, 0.0));
    }

    pub fn mouse_down(&mut self, point: Point) {
        self.event(Event::MouseDown(point))
    }

    fn event(&mut self, event: Event) {
        self.root.event(&event, self.root_id, &mut self.actions);
        self.run_app_logic();
    }

    pub(crate) fn run_app_logic(&mut self) {
        let actions = std::mem::take(&mut self.actions);
        let mutation = (self.app_logic)(actions);
        self.root.mutate(mutation);
    }
}
