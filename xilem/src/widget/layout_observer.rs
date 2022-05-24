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

//! The widget-side implementation of layout observers.
//!
//! This concept is very similar to GeometryReader in SwiftUI.

use druid_shell::kurbo::Size;

use crate::{event::Event, id::IdPath};

use super::{
    align::SingleAlignment, contexts::LifeCycleCx, AlignCx, AnyWidget, EventCx, LayoutCx,
    LifeCycle, PaintCx, Pod, RawEvent, UpdateCx, Widget,
};

pub struct LayoutObserver {
    id_path: IdPath,
    size: Option<Size>,
    child: Option<Pod>,
}

impl LayoutObserver {
    pub fn new(id_path: &IdPath) -> LayoutObserver {
        LayoutObserver {
            id_path: id_path.clone(),
            size: None,
            child: None,
        }
    }

    pub fn set_child(&mut self, child: Box<dyn AnyWidget>) {
        self.child = Some(Pod::new_from_box(child));
    }

    pub fn child_mut(&mut self) -> &mut Option<Pod> {
        &mut self.child
    }
}

impl Widget for LayoutObserver {
    fn update(&mut self, cx: &mut UpdateCx) {
        // Need to make sure we do layout on child when set.
        cx.request_layout();
        if let Some(child) = &mut self.child {
            child.update(cx);
        }
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        if let Some(child) = &mut self.child {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        if let Some(child) = &mut self.child {
            child.lifecycle(cx, event);
        }
    }

    fn measure(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        if let Some(child) = &mut self.child {
            let _ = child.measure(cx);
        }
        (Size::ZERO, Size::new(1e9, 1e9))
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        if Some(proposed_size) != self.size {
            cx.add_event(Event::new(self.id_path.clone(), proposed_size));
            self.size = Some(proposed_size);
        }
        if let Some(child) = &mut self.child {
            let _ = child.layout(cx, proposed_size);
        }
        proposed_size
    }

    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        if let Some(child) = &self.child {
            child.align(cx, alignment);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        if let Some(child) = &mut self.child {
            child.paint(cx);
        }
    }
}
