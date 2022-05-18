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

//! Core types and mechanisms for the widget hierarchy.
//!
//! //! Note: the organization of this code roughly follows the existing Druid
//! widget system, particularly its core.rs.

use bitflags::bitflags;
use druid_shell::{
    kurbo::{Affine, Point, Size},
    piet::RenderContext,
};

use crate::Widget;

use super::{
    align::{
        AlignResult, AlignmentAxis, Bottom, Center, HorizAlignment, Leading, SingleAlignment, Top,
        Trailing, VertAlignment,
    },
    AlignCx, AnyWidget, EventCx, LayoutCx, PaintCx, RawEvent, UpdateCx,
};

bitflags! {
    #[derive(Default)]
    pub(crate) struct PodFlags: u32 {
        const REQUEST_UPDATE = 1;
        const REQUEST_LAYOUT = 2;
        const REQUEST_PAINT = 4;

        const UPWARD_FLAGS = Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
        const INIT_FLAGS = Self::REQUEST_UPDATE.bits | Self::REQUEST_LAYOUT.bits | Self::REQUEST_PAINT.bits;
    }
}

/// A pod that contains a widget (in a container).
pub struct Pod {
    pub(crate) state: WidgetState,
    pub(crate) widget: Box<dyn AnyWidget>,
}

#[derive(Default, Debug)]
pub(crate) struct WidgetState {
    pub(crate) flags: PodFlags,
    pub(crate) origin: Point,
    /// The minimum intrinsic size of the widget.
    pub(crate) min_size: Size,
    /// The maximum intrinsic size of the widget.
    pub(crate) max_size: Size,
    /// The size proposed by the widget's container.
    pub(crate) proposed_size: Size,
    /// The size of the widget.
    pub(crate) size: Size,
}

impl WidgetState {
    fn merge_up(&mut self, child_state: &mut WidgetState) {
        self.flags |= child_state.flags & PodFlags::UPWARD_FLAGS;
    }

    fn request(&mut self, flags: PodFlags) {
        self.flags |= flags
    }

    /// Get alignment value.
    ///
    /// The value is in the coordinate system of the parent widget.
    pub(crate) fn get_alignment(&self, widget: &dyn AnyWidget, alignment: SingleAlignment) -> f64 {
        if alignment.id() == Leading.id() || alignment.id() == Top.id() {
            0.0
        } else if alignment.id() == <Center as HorizAlignment>::id(&Center) {
            match alignment.axis() {
                AlignmentAxis::Horizontal => self.size.width * 0.5,
                AlignmentAxis::Vertical => self.size.height * 0.5,
            }
        } else if alignment.id() == Trailing.id() {
            self.size.width
        } else if alignment.id() == Bottom.id() {
            self.size.height
        } else {
            let mut align_result = AlignResult::default();
            let mut align_cx = AlignCx {
                widget_state: self,
                align_result: &mut align_result,
                origin: self.origin,
            };
            widget.align(&mut align_cx, alignment);
            align_result.reap(alignment)
        }
    }
}

impl Pod {
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self::new_from_box(Box::new(widget))
    }

    pub fn new_from_box(widget: Box<dyn AnyWidget>) -> Self {
        Pod {
            state: WidgetState {
                flags: PodFlags::INIT_FLAGS,
                ..Default::default()
            },
            widget,
        }
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        (*self.widget).as_any_mut().downcast_mut()
    }

    pub fn request_update(&mut self) {
        self.state.request(PodFlags::REQUEST_UPDATE);
    }

    pub fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.widget.event(cx, event);
    }

    /// Propagate an update cycle.
    pub fn update(&mut self, cx: &mut UpdateCx) {
        if self.state.flags.contains(PodFlags::REQUEST_UPDATE) {
            let mut child_cx = UpdateCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            self.widget.update(&mut child_cx);
            self.state.flags.remove(PodFlags::REQUEST_UPDATE);
            cx.widget_state.merge_up(&mut self.state);
        }
    }

    pub fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        if self.state.flags.contains(PodFlags::REQUEST_LAYOUT) {
            let mut child_cx = LayoutCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            let (min_size, max_size) = self.widget.prelayout(&mut child_cx);
            self.state.min_size = min_size;
            self.state.max_size = max_size;
            // Don't remove REQUEST_LAYOUT here, that will be done in layout.
        }
        (self.state.min_size, self.state.max_size)
    }

    pub fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        if self.state.flags.contains(PodFlags::REQUEST_LAYOUT)
            || proposed_size != self.state.proposed_size
        {
            let mut child_cx = LayoutCx {
                cx_state: cx.cx_state,
                widget_state: &mut self.state,
            };
            let new_size = self.widget.layout(&mut child_cx, proposed_size);
            self.state.proposed_size = proposed_size;
            self.state.size = new_size;
            self.state.flags.remove(PodFlags::REQUEST_LAYOUT);
        }
        self.state.size
    }

    /// Propagate alignment query to children.
    ///
    /// This call aggregates all instances of the alignment, so cost may be
    /// proportional to the number of descendants.
    pub fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        let mut child_cx = AlignCx {
            widget_state: &self.state,
            align_result: cx.align_result,
            origin: cx.origin + self.state.origin.to_vec2(),
        };
        self.widget.align(&mut child_cx, alignment);
    }

    pub fn paint(&mut self, cx: &mut PaintCx) {
        cx.with_save(|cx| {
            cx.piet
                .transform(Affine::translate(self.state.origin.to_vec2()));
            self.widget.paint(cx);
        });
    }

    pub fn height_flexibility(&self) -> f64 {
        self.state.max_size.height - self.state.min_size.height
    }

    /// The returned value is in the coordinate space of the parent that
    /// owns this pod.
    pub fn get_alignment(&self, alignment: SingleAlignment) -> f64 {
        self.state.get_alignment(&self.widget, alignment)
    }
}
