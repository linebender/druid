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

use crate::kurbo::Size;
use crate::widget::Widget;
use crate::{BoxConstraints, LayoutResult};
use crate::{Id, LayoutCtx, Ui};

/// A padding widget. Is expected to have exactly one child.
pub struct Padding {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
}

impl Padding {
    /// Create widget with uniform padding.
    pub fn uniform(padding: f64) -> Padding {
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
        size: Option<Size>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        let hpad = self.left + self.right;
        let vpad = self.top + self.bottom;
        if let Some(size) = size {
            ctx.position_child(children[0], (self.left, self.top));
            LayoutResult::Size(Size::new(size.width + hpad, size.height + vpad))
        } else {
            let min = Size::new(bc.min.width - hpad, bc.min.height - hpad);
            let max = Size::new(bc.max.width - hpad, bc.max.height - hpad);
            LayoutResult::RequestChild(children[0], BoxConstraints::new(min, max))
        }
    }
}
