// Copyright 2018 Google LLC
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

use {BoxConstraints, LayoutResult};
use {Id, LayoutCtx, UiInner};
use widget::Widget;

pub struct Row;
pub struct Column;

pub struct Flex {
    direction: Axis,

    // layout continuation state
    ix: usize,
    major_per_flex: f32,
    minor: f32,
}

pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn major(&self, coords: (f32, f32)) -> f32 {
        match *self {
            Axis::Horizontal => coords.0,
            Axis::Vertical => coords.1,
        }
    }

    fn minor(&self, coords: (f32, f32)) -> f32 {
        match *self {
            Axis::Horizontal => coords.1,
            Axis::Vertical => coords.0,
        }
    }

    fn pack(&self, major: f32, minor: f32) -> (f32, f32) {
        match *self {
            Axis::Horizontal => (major, minor),
            Axis::Vertical => (minor, major),
        }
    }
}

impl Row {
    pub fn new() -> Flex {
        Flex {
            direction: Axis::Horizontal,

            ix: 0,
            major_per_flex: 0.0,
            minor: 0.0,
        }
    }
}

impl Column {
    pub fn new() -> Flex {
        Flex {
            direction: Axis::Vertical,

            ix: 0,
            major_per_flex: 0.0,
            minor: 0.0,
        }
    }
}

impl Flex {
    pub fn ui(self, children: &[Id], ctx: &mut UiInner) -> Id {
        ctx.add(self, children)
    }
}

impl Widget for Flex {
    fn layout(&mut self, bc: &BoxConstraints, children: &[Id], size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx) -> LayoutResult
    {
        if let Some(size) = size {
            let minor = self.direction.minor(size);
            self.minor = self.minor.max(minor);
            self.ix += 1;
            if self.ix == children.len() {
                // measured all children
                let mut major = 0.0;
                for &child in children {
                    // top-align, could do center etc. based on child height
                    ctx.position_child(child, self.direction.pack(major, 0.0));
                    major += self.major_per_flex;
                }
                let max_major = self.direction.major((bc.max_width, bc.max_height));
                return LayoutResult::Size(self.direction.pack(max_major, self.minor));
            }
        } else {
            if children.is_empty() {
                return LayoutResult::Size((bc.min_width, bc.min_height));
            }
            self.ix = 0;
            self.minor = self.direction.minor((bc.min_width, bc.min_height));
            let max_major = self.direction.major((bc.max_width, bc.max_height));
            self.major_per_flex = max_major / (children.len() as f32);
        }
        let child_bc = match self.direction {
            Axis::Horizontal => BoxConstraints {
                min_width: self.major_per_flex,
                max_width: self.major_per_flex,
                min_height: bc.min_height,
                max_height: bc.max_height,
            },
            Axis::Vertical => BoxConstraints {
                min_width: bc.min_width,
                max_width: bc.max_width,
                min_height: self.major_per_flex,
                max_height: self.major_per_flex,
            },
        };
        LayoutResult::RequestChild(children[self.ix], child_bc)
    }
}
