use super::{AlignCx, AnyWidget, Widget, WidgetState};

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

#[derive(Clone, Copy, PartialEq)]
pub enum AlignmentMerge {
    Min,
    Mean,
    Max,
}

pub trait HorizAlignment: 'static {
    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Mean
    }
}

pub trait VertAlignment: 'static {
    fn id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }

    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Mean
    }
}

pub struct Leading;

impl HorizAlignment for Leading {
    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Min
    }
}
pub struct HorizCenter;

impl HorizAlignment for HorizCenter {}

pub struct Trailing;

impl HorizAlignment for Trailing {
    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Max
    }
}

pub struct Top;

impl VertAlignment for Top {
    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Min
    }
}

pub struct VertCenter;

impl VertAlignment for VertCenter {}

pub struct Bottom;

impl VertAlignment for Bottom {
    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Max
    }
}

pub struct FirstBaseline;

impl VertAlignment for FirstBaseline {
    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Min
    }
}

pub struct LastBaseline;

impl VertAlignment for LastBaseline {
    fn merge(&self) -> AlignmentMerge {
        AlignmentMerge::Max
    }
}

#[derive(Clone, Copy)]
pub struct OneAlignment {
    pub id: std::any::TypeId,
    merge: AlignmentMerge,
}

impl OneAlignment {
    pub fn from_horiz(h: &impl HorizAlignment) -> OneAlignment {
        OneAlignment {
            id: h.id(),
            merge: h.merge(),
        }
    }

    pub fn from_vert(v: &impl VertAlignment) -> OneAlignment {
        OneAlignment {
            id: v.id(),
            merge: v.merge(),
        }
    }
}

#[derive(Default)]
pub struct AlignResult {
    value: f64,
    count: usize,
}

impl AlignResult {
    pub fn aggregate(&mut self, alignment: OneAlignment, value: f64) {
        match alignment.merge {
            AlignmentMerge::Max => {
                if self.count == 0 {
                    self.value = value;
                } else {
                    self.value = self.value.max(value)
                }
            }
            AlignmentMerge::Min => {
                if self.count == 0 {
                    self.value = value;
                } else {
                    self.value = self.value.min(value)
                }
            }
            AlignmentMerge::Mean => self.value += value,
        }
        self.count += 1;
    }

    pub fn reap(&self, alignment: OneAlignment) -> f64 {
        match alignment.merge {
            AlignmentMerge::Mean => {
                if self.count == 0 {
                    0.0
                } else {
                    self.value / self.count as f64
                }
            }
            _ => self.value,
        }
    }
}
// AlignmentGuide widget

/// A proxy that can be queried for alignments.
struct AlignmentProxy<'a> {
    widget_state: &'a WidgetState,
    widget: &'a dyn AnyWidget,
}

struct AlignmentGuide<F> {
    alignment_id: std::any::TypeId,
    callback: F,
    child: Box<dyn AnyWidget>,
}

impl<'a> AlignmentProxy<'a> {
    pub fn get_alignment(&self, alignment: OneAlignment) -> f64 {
        self.widget_state.get_alignment(self.widget, alignment)
    }

    pub fn width(&self) -> f64 {
        self.widget_state.size.width
    }

    pub fn height(&self) -> f64 {
        self.widget_state.size.height
    }
}

impl<F: Fn(AlignmentProxy) -> f64 + 'static> Widget for AlignmentGuide<F> {
    fn event(&mut self, event: &super::RawEvent, events: &mut Vec<crate::event::Event>) {
        self.child.event(event, events);
    }

    fn update(&mut self, cx: &mut super::UpdateCx) {
        self.child.update(cx);
    }

    fn prelayout(
        &mut self,
        cx: &mut super::LayoutCx,
    ) -> (druid_shell::kurbo::Size, druid_shell::kurbo::Size) {
        self.child.prelayout(cx)
    }

    fn layout(
        &mut self,
        cx: &mut super::LayoutCx,
        proposed_size: druid_shell::kurbo::Size,
    ) -> druid_shell::kurbo::Size {
        self.child.layout(cx, proposed_size)
    }

    fn align(&self, cx: &mut AlignCx, alignment: OneAlignment) {
        if alignment.id == self.alignment_id {
            let proxy = AlignmentProxy {
                widget_state: cx.widget_state,
                widget: self,
            };
            let value = (self.callback)(proxy);
            cx.align_result.aggregate(alignment, value);
        } else {
            self.child.align(cx, alignment);
        }
    }

    fn paint(&mut self, cx: &mut super::PaintCx) {
        self.child.paint(cx);
    }
}
