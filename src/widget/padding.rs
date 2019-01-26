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

//! A widget that just adds padding during layout.

use widget::Widget;
use {BoxConstraints, LayoutResult};
use {Id, LayoutCtx, Ui};

/// A padding widget. Is expected to have exactly one child.
pub struct Padding {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl Padding {
    /// Create widget with uniform padding.
    pub fn uniform(padding: f32) -> Padding {
        Padding {
            left: padding,
            right: padding,
            top: padding,
            bottom: padding,
        }
    }

    pub fn ui(self, child: Id, ctx: &mut Ui) -> Id {
        ctx.add(self, &[child])
    }
}

impl Widget for Padding {
    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        if let Some(size) = size {
            ctx.position_child(children[0], (self.left, self.top));
            LayoutResult::Size((
                size.0 + self.left + self.right,
                size.1 + self.top + self.bottom,
            ))
        } else {
            let child_bc = BoxConstraints {
                min_width: bc.min_width - (self.left + self.right),
                max_width: bc.max_width - (self.left + self.right),
                min_height: bc.min_height - (self.top + self.bottom),
                max_height: bc.max_height - (self.top + self.bottom),
            };
            LayoutResult::RequestChild(children[0], child_bc)
        }
    }
}
