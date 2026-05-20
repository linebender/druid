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

use druid_shell::{
    kurbo::{Point, Size, Vec2},
    piet::Piet,
};

use crate::event::Event;

use super::{Geom, RawEvent, Widget, WidgetTuple};

pub struct Column<W: WidgetTuple> {
    children: W,
    geoms: Vec<Geom>,
}

impl<W: WidgetTuple> Column<W> {
    pub fn new(children: W) -> Self {
        let geoms = (0..children.length()).map(|_| Geom::default()).collect();
        Column { children, geoms }
    }

    pub fn children_mut(&mut self) -> &mut W {
        &mut self.children
    }
}

impl<W: WidgetTuple> Widget for Column<W> {
    fn event(&mut self, event: &super::RawEvent, events: &mut Vec<Event>) {
        match event {
            RawEvent::MouseDown(p) => {
                let mut p = *p;
                for (child, geom) in self.children.widgets_mut().into_iter().zip(&self.geoms) {
                    if p.y < geom.size.height {
                        let child_event = RawEvent::MouseDown(p);
                        child.event(&child_event, events);
                        break;
                    }
                    p.y -= geom.size.height;
                }
            }
        }
    }

    fn layout(&mut self) -> Size {
        let mut size = Size::default();
        for (child, geom) in self.children.widgets_mut().into_iter().zip(&mut self.geoms) {
            let child_size = child.layout();
            geom.size = child_size;
            size.width = size.width.max(child_size.width);
            size.height += child_size.height;
        }
        size
    }

    fn paint(&mut self, ctx: &mut Piet, pos: Point) {
        let mut child_pos = pos + Vec2::new(10.0, 0.0);
        for (child, geom) in self.children.widgets_mut().into_iter().zip(&self.geoms) {
            child.paint(ctx, child_pos);
            child_pos.y += geom.size.height;
        }
    }
}
