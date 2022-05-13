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

use druid_shell::kurbo::{Point, Rect, Size};

use crate::event::Event;

use super::{
    align::{HorizCenter, SingleAlignment},
    LayoutCx, PaintCx, Pod, RawEvent, UpdateCx, Widget,
};

pub struct VStack {
    children: Vec<Pod>,
    alignment: SingleAlignment,
    spacing: f64,
}

impl VStack {
    pub fn new(children: Vec<Pod>) -> Self {
        let alignment = SingleAlignment::from_horiz(&HorizCenter);
        let spacing = 0.0;
        VStack {
            children,
            alignment,
            spacing,
        }
    }

    pub fn children_mut(&mut self) -> &mut Vec<Pod> {
        &mut self.children
    }
}

impl Widget for VStack {
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

    fn update(&mut self, cx: &mut UpdateCx) {
        for child in &mut self.children {
            child.update(cx);
        }
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        let mut min_size = Size::ZERO;
        let mut max_size = Size::ZERO;
        for child in &mut self.children {
            let (child_min, child_max) = child.prelayout(cx);
            min_size.width = min_size.width.max(child_min.width);
            min_size.height += child_min.height;
            max_size.width = max_size.width.max(child_max.width);
            max_size.height += child_max.height;
        }
        (min_size, max_size)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        // First, sort children in order of increasing flexibility
        let mut child_order: Vec<_> = (0..self.children.len()).collect();
        child_order.sort_by_key(|ix| self.children[*ix].height_flexibility().to_bits());
        let mut n_remaining = self.children.len();
        let mut height_remaining = proposed_size.height - (n_remaining - 1) as f64 * self.spacing;
        for ix in child_order {
            let child_height = (height_remaining / n_remaining as f64).max(0.0);
            let child_proposed = Size::new(proposed_size.width, child_height);
            let child_size = self.children[ix].layout(cx, child_proposed);
            height_remaining -= height_remaining - child_size.height;
            n_remaining -= 1;
        }
        let alignments: Vec<f64> = self
            .children
            .iter()
            .map(|child| child.get_alignment(self.alignment))
            .collect();
        let max_align = alignments
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or_default();
        let mut size = Size::default();
        let mut y = 0.0;
        for (child, align) in self.children.iter_mut().zip(alignments) {
            let child_size = child.state.size;
            child.state.origin = Point::new(max_align - align, y);
            size.width = size.width.max(child_size.width);
            size.height += child_size.height;
            y += child_size.height + self.spacing;
        }
        size
    }

    fn align(&self, cx: &mut super::AlignCx, alignment: SingleAlignment) {
        for child in &self.children {
            child.align(cx, alignment);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        for child in &mut self.children {
            child.paint(cx);
        }
    }
}
