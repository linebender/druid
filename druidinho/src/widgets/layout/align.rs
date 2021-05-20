// Copyright 2018 The Druid Authors.
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

//! A widget that aligns its child (for example, centering it).

use super::LayoutHost;
use crate::kurbo::{Rect, Size};
use crate::piet::UnitPoint;
use crate::widget::SingleChildContainer;
use crate::{BoxConstraints, LayoutCtx, Widget};

/// A widget that aligns its child.
pub struct Align<T> {
    align: UnitPoint,
    child: LayoutHost<T>,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

impl<T> Align<T> {
    /// Create widget with alignment.
    ///
    /// Note that the `align` parameter is specified as a `UnitPoint` in
    /// terms of left and right. This is inadequate for bidi-aware layout
    /// and thus the API will change when druid gains bidi capability.
    pub fn new(child: T) -> Align<T> {
        Align {
            align: UnitPoint::TOP_LEFT,
            child: LayoutHost::new(child),
            width_factor: None,
            height_factor: None,
        }
    }

    /// Create centered widget.
    pub fn centered(mut self) -> Self {
        self.align = UnitPoint::CENTER;
        self
    }

    /// Create right-aligned widget.
    pub fn right(mut self) -> Self {
        self.align = UnitPoint::RIGHT;
        self
    }

    /// Create left-aligned widget.
    pub fn left(mut self) -> Self {
        self.align = UnitPoint::LEFT;
        self
    }
}

impl<W: Widget> SingleChildContainer for Align<W> {
    type Child = LayoutHost<W>;

    fn widget(&self) -> &Self::Child {
        &self.child
    }

    fn widget_mut(&mut self) -> &mut Self::Child {
        &mut self.child
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        bc.debug_check("Align");

        let size = SingleChildContainer::layout(&mut self.child, ctx, bc.loosen());
        log_size_warnings(size);

        let mut my_size = size;
        if bc.is_width_bounded() {
            my_size.width = bc.max().width;
        }
        if bc.is_height_bounded() {
            my_size.height = bc.max().height;
        }

        if let Some(width) = self.width_factor {
            my_size.width = size.width * width;
        }
        if let Some(height) = self.height_factor {
            my_size.height = size.height * height;
        }

        my_size = bc.constrain(my_size);
        let extra_width = (my_size.width - size.width).max(0.);
        let extra_height = (my_size.height - size.height).max(0.);
        let origin = self
            .align
            .resolve(Rect::new(0., 0., extra_width, extra_height))
            .expand();
        self.child.set_origin(origin);
        bc.constrain(my_size)
    }
}

fn log_size_warnings(size: Size) {
    if size.width.is_infinite() {
        eprintln!("Align widget's child has an infinite width.");
    }

    if size.height.is_infinite() {
        eprintln!("Align widget's child has an infinite height.");
    }
}
