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

#[derive(Clone, PartialEq, Debug)]
pub enum HorizAlignment {
    // Note: actually "left" until we do BiDi.
    Leading,
    Center,
    Trailing,
    // We might switch to TinyStr.
    Custom(&'static str),
}

#[derive(Clone, PartialEq, Debug)]
pub enum VertAlignment {
    Top,
    Center,
    Bottom,
    FirstBaseline,
    LastBaseline,
    Custom(&'static str),
}

#[derive(Clone, PartialEq, Debug)]
pub enum OneAlignment {
    Horiz(HorizAlignment),
    Vert(VertAlignment),
}

#[derive(Default)]
pub struct AlignResult {
    value: f64,
    count: usize,
}

impl HorizAlignment {
    fn aggregate(&self, result: &mut AlignResult, value: f64) {
        result.count += 1;
        match self {
            HorizAlignment::Leading => result.value = result.value.min(value),
            HorizAlignment::Trailing => result.value = result.value.max(value),
            _ => result.value += value,
        }
    }
}

impl VertAlignment {
    fn aggregate(&self, result: &mut AlignResult, value: f64) {
        result.count += 1;
        match self {
            VertAlignment::Top | VertAlignment::FirstBaseline => {
                result.value = result.value.min(value)
            }
            VertAlignment::Bottom | VertAlignment::LastBaseline => {
                result.value = result.value.max(value)
            }
            _ => result.value += value,
        }
    }
}

impl OneAlignment {
    pub fn aggregate(&self, result: &mut AlignResult, value: f64) {
        match self {
            Self::Horiz(h) => h.aggregate(result, value),
            Self::Vert(v) => v.aggregate(result, value),
        }
    }
}

impl AlignResult {
    pub fn reap(&self, alignment: &OneAlignment) -> f64 {
        match alignment {
            OneAlignment::Horiz(HorizAlignment::Center)
            | OneAlignment::Horiz(HorizAlignment::Custom(_))
            | OneAlignment::Vert(VertAlignment::Center)
            | OneAlignment::Vert(VertAlignment::Custom(_)) => {
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
    alignment: OneAlignment,
    callback: F,
    child: Box<dyn AnyWidget>,
}

impl<'a> AlignmentProxy<'a> {
    pub fn get_alignment(&self, alignment: &OneAlignment) -> f64 {
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

    fn align(&self, cx: &mut AlignCx, alignment: &OneAlignment) {
        if *alignment == self.alignment {
            let proxy = AlignmentProxy {
                widget_state: cx.widget_state,
                widget: self,
            };
            let value = (self.callback)(proxy);
            alignment.aggregate(cx.align_result, value);
        } else {
            self.child.align(cx, alignment);
        }
    }

    fn paint(&mut self, cx: &mut super::PaintCx) {
        self.child.paint(cx);
    }
}
