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
    kurbo::{Point, Rect, Size, Vec2},
    piet::Piet,
};

use crate::event::Event;

use super::{Geom, LayoutCx, PaintCx, Pod, RawEvent, Widget};

pub struct Column {
    children: Vec<Pod>,
}

impl Column {
    pub fn new(children: Vec<Pod>) -> Self {
        Column { children }
    }

    pub fn children_mut(&mut self) -> &mut Vec<Pod> {
        &mut self.children
    }
}

impl Widget for Column {
    fn event(&mut self, event: &super::RawEvent, events: &mut Vec<Event>) {
        match event {
            RawEvent::MouseDown(p) => {
                for child in &mut self.children {
                    let rect = Rect::from_origin_size(child.state.origin, child.state.size);
                    if rect.contains(*p) {
                        let child_event = RawEvent::MouseDown(*p - child.state.origin.to_vec2());
                        child.event(&child_event, events);
                        break;
                    }
                }
            }
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        let mut size = Size::default();
        let mut offset = Point::ZERO;
        for child in &mut self.children {
            let child_size = child.layout(cx, proposed_size);
            child.state.origin = offset;
            size.width = size.width.max(child_size.width);
            size.height += child_size.height;
            offset.y += child_size.height;
        }
        size
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}
