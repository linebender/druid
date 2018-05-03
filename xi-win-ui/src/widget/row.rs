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

//! A button widget

use {BoxConstraints, LayoutResult};
use {Id, LayoutCtx};
use widget::Widget;

#[derive(Default)]
pub struct Row {
    // layout continuation state
    ix: usize,
    width_per_flex: f32,
    height: f32,
}

impl Widget for Row {
    fn layout(&mut self, bc: &BoxConstraints, children: &[Id], size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx) -> LayoutResult
    {
        if let Some(size) = size {
            if size.1 > self.height {
                self.height = size.1;
            }
            self.ix += 1;
            if self.ix == children.len() {
                // measured all children
                let mut x = 0.0;
                for &child in children {
                    // top-align, could do center etc. based on child height
                    ctx.position_child(child, (x, 0.0));
                    x += self.width_per_flex;
                }
                return LayoutResult::Size((bc.max_width, self.height));
            }
        } else {
            if children.is_empty() {
                return LayoutResult::Size((bc.min_width, bc.min_height));
            }
            self.ix = 0;
            self.height = bc.min_height;
            self.width_per_flex = bc.max_width / (children.len() as f32);
        }
        let child_bc = BoxConstraints {
            min_width: self.width_per_flex,
            max_width: self.width_per_flex,
            min_height: bc.min_height,
            max_height: bc.max_height,
        };
        LayoutResult::RequestChild(children[self.ix], child_bc)
    }
}
