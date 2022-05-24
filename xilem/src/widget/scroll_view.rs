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

//! A simple scroll view.
//!
//! There's a lot more functionality in the Druid version, including
//! control over scrolling axes, ability to scroll to content, etc.

use druid_shell::{
    kurbo::{Affine, Point, Rect, Size, Vec2},
    piet::RenderContext,
};

use crate::Widget;

use super::{
    contexts::LifeCycleCx, EventCx, LayoutCx, LifeCycle, PaintCx, Pod, RawEvent, UpdateCx,
};

pub struct ScrollView {
    child: Pod,
    offset: f64,
}

impl ScrollView {
    pub fn new(child: impl Widget + 'static) -> Self {
        ScrollView {
            child: Pod::new(child),
            offset: 0.0,
        }
    }

    pub fn child_mut(&mut self) -> &mut Pod {
        &mut self.child
    }
}

impl Widget for ScrollView {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        // TODO: scroll wheel + click-drag on scroll bars
        let offset = Vec2::new(0.0, self.offset);
        let child_event = match event {
            RawEvent::MouseDown(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                RawEvent::MouseDown(mouse_event)
            }
            RawEvent::MouseUp(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                RawEvent::MouseUp(mouse_event)
            }
            RawEvent::MouseMove(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                RawEvent::MouseMove(mouse_event)
            }
            RawEvent::MouseWheel(mouse_event) => {
                let mut mouse_event = mouse_event.clone();
                mouse_event.pos += offset;
                RawEvent::MouseWheel(mouse_event)
            }
            _ => event.clone(),
        };
        self.child.event(cx, &child_event);
        if !cx.is_handled() {
            if let RawEvent::MouseWheel(mouse) = event {
                let new_offset = (self.offset + mouse.wheel_delta.y).max(0.0);
                if new_offset != self.offset {
                    self.offset = new_offset;
                    cx.set_handled(true);
                    // TODO: request paint
                }
            }
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.child.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.child.update(cx);
    }

    fn measure(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        let _ = self.child.measure(cx);
        (Size::ZERO, Size::new(1e9, 1e9))
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        let child_proposed = Size::new(proposed_size.width, 1e9);
        self.child.layout(cx, child_proposed)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        cx.with_save(|cx| {
            let size = cx.size();
            cx.clip(Rect::from_origin_size(Point::ZERO, size));
            cx.transform(Affine::translate((0.0, -self.offset)));
            self.child.paint_raw(cx);
        });
    }
}
