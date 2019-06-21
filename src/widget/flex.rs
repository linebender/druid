// Copyright 2018 The xi-editor Authors.
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

//! A widget that arranges its children in a one-dimensional array.

use std::collections::BTreeMap;

use crate::kurbo::Size;

use crate::widget::Widget;
use crate::{BoxConstraints, LayoutResult};
use crate::{Id, LayoutCtx, Ui};

pub struct Row;
pub struct Column;

pub struct Flex {
    params: BTreeMap<Id, Params>,
    direction: Axis,

    // layout continuation state
    phase: Phase,
    ix: usize,
    minor: f64,

    // the total measure of non-flex children
    total_non_flex: f64,

    // the sum of flex parameters of all children
    flex_sum: f64,
}

pub enum Axis {
    Horizontal,
    Vertical,
}

// Layout happens in two phases. First, the non-flex children
// are laid out. Then, the remaining space is divided across
// the flex children.
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    NonFlex,
    Flex,
}

#[derive(Copy, Clone, Default)]
struct Params {
    flex: f64,
}

impl Params {
    // Determine the phase in which this child should be measured.
    fn get_flex_phase(&self) -> Phase {
        if self.flex == 0.0 {
            Phase::NonFlex
        } else {
            Phase::Flex
        }
    }
}

impl Axis {
    fn major(&self, coords: Size) -> f64 {
        match *self {
            Axis::Horizontal => coords.width,
            Axis::Vertical => coords.height,
        }
    }

    fn minor(&self, coords: Size) -> f64 {
        match *self {
            Axis::Horizontal => coords.height,
            Axis::Vertical => coords.width,
        }
    }

    fn pack(&self, major: f64, minor: f64) -> (f64, f64) {
        match *self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }
}

impl Row {
    pub fn new() -> Flex {
        Flex {
            params: BTreeMap::new(),
            direction: Axis::Horizontal,

            phase: Phase::NonFlex,
            ix: 0,
            minor: 0.0,
            total_non_flex: 0.0,
            flex_sum: 0.0,
        }
    }
}

impl Column {
    pub fn new() -> Flex {
        Flex {
            params: BTreeMap::new(),
            direction: Axis::Vertical,

            phase: Phase::NonFlex,
            ix: 0,
            minor: 0.0,
            total_non_flex: 0.0,
            flex_sum: 0.0,
        }
    }
}

impl Flex {
    /// Add to UI with children.
    pub fn ui(self, children: &[Id], ctx: &mut Ui) -> Id {
        ctx.add(self, children)
    }

    /// Set the flex for a child widget.
    ///
    /// This function is used to set flex for a child widget, and is done while
    /// building, before adding to the UI. Likely we will need to think of other
    /// mechanisms to change parameters dynamically after building.
    pub fn set_flex(&mut self, child: Id, flex: f64) {
        let params = self.get_params_mut(child);
        params.flex = flex;
    }

    fn get_params_mut(&mut self, child: Id) -> &mut Params {
        self.params.entry(child).or_default()
    }

    fn get_params(&self, child: Id) -> Params {
        self.params
            .get(&child)
            .cloned()
            .unwrap_or(Default::default())
    }

    /// Return the index (within `children`) of the next child that belongs in
    /// the specified phase.
    fn get_next_child(&self, children: &[Id], start: usize, phase: Phase) -> Option<usize> {
        for ix in start..children.len() {
            if self.get_params(children[ix]).get_flex_phase() == phase {
                return Some(ix);
            }
        }
        None
    }

    /// Position all children, after the children have all been measured.
    fn finish_layout(
        &self,
        bc: &BoxConstraints,
        children: &[Id],
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        let mut major = 0.0;
        for &child in children {
            // top-align, could do center etc. based on child height
            ctx.position_child(child, self.direction.pack(major, 0.0));
            major += self.direction.major(ctx.get_child_size(child));
        }
        let total_major = self.direction.major(bc.max);
        let (width, height) = self.direction.pack(total_major, self.minor);
        LayoutResult::Size(Size::new(width, height))
    }
}

impl Widget for Flex {
    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<Size>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        if let Some(size) = size {
            let minor = self.direction.minor(size);
            self.minor = self.minor.max(minor);
            if self.phase == Phase::NonFlex {
                self.total_non_flex += self.direction.major(size);
            }

            // Advance to the next child; finish non-flex phase if at end.
            if let Some(ix) = self.get_next_child(children, self.ix + 1, self.phase) {
                self.ix = ix;
            } else if self.phase == Phase::NonFlex {
                if let Some(ix) = self.get_next_child(children, 0, Phase::Flex) {
                    self.ix = ix;
                    self.phase = Phase::Flex;
                } else {
                    return self.finish_layout(bc, children, ctx);
                }
            } else {
                return self.finish_layout(bc, children, ctx);
            }
        } else {
            // Start layout process, no children measured yet.
            if children.is_empty() {
                return LayoutResult::Size(bc.min);
            }
            if let Some(ix) = self.get_next_child(children, 0, Phase::NonFlex) {
                self.ix = ix;
                self.phase = Phase::NonFlex;
            } else {
                // All children are flex, skip non-flex pass.
                self.ix = 0;
                self.phase = Phase::Flex;
            }
            self.total_non_flex = 0.0;
            self.flex_sum = children.iter().map(|id| self.get_params(*id).flex).sum();
            self.minor = self.direction.minor(bc.min);
        }
        let (min_major, max_major) = if self.phase == Phase::NonFlex {
            (0.0, ::std::f64::INFINITY)
        } else {
            let total_major = self.direction.major(bc.max);
            // TODO: should probably max with 0.0 to avoid negative sizes
            let remaining = total_major - self.total_non_flex;
            let major = remaining * self.get_params(children[self.ix]).flex / self.flex_sum;
            (major, major)
        };
        let child_bc = match self.direction {
            Axis::Horizontal => BoxConstraints::new(
                Size::new(min_major, bc.min.height),
                Size::new(max_major, bc.max.height),
            ),
            Axis::Vertical => BoxConstraints::new(
                Size::new(bc.min.width, min_major),
                Size::new(bc.max.width, max_major),
            ),
        };
        LayoutResult::RequestChild(children[self.ix], child_bc)
    }

    fn on_child_removed(&mut self, child: Id) {
        self.params.remove(&child);
    }
}
