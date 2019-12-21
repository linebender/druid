// Copyright 2019 The xi-editor Authors.
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

//! The fundamental druid types.

use crate::kurbo::Size;
use log;

/// Constraints for layout.
///
/// The layout strategy for druid is strongly inspired by Flutter,
/// and this struct is similar to the [Flutter BoxConstraints] class.
///
/// At the moment, it represents simply a minimum and maximum size.
/// A widget's [`layout`] method should choose an appropriate size that
/// meets these constraints.
///
/// Further, a container widget should compute appropriate constraints
/// for each of its child widgets, and pass those down when recursing.
///
/// [`layout`]: trait.Widget.html#tymethod.layout
/// [Flutter BoxConstraints]: https://api.flutter.dev/flutter/rendering/BoxConstraints-class.html
#[derive(Clone, Copy, Debug)]
pub struct BoxConstraints {
    min: Size,
    max: Size,
}

impl BoxConstraints {
    /// Create a new box constraints object.
    ///
    /// Create constraints based on minimum and maximum size.
    pub fn new(min: Size, max: Size) -> BoxConstraints {
        BoxConstraints { min, max }
    }

    /// Create a "tight" box constraints object.
    ///
    /// A "tight" constraint can only be satisfied by a single size.
    pub fn tight(size: Size) -> BoxConstraints {
        BoxConstraints {
            min: size,
            max: size,
        }
    }

    /// Create a "loose" version of the constraints.
    ///
    /// Make a version with zero minimum size, but the same maximum size.
    pub fn loosen(&self) -> BoxConstraints {
        BoxConstraints {
            min: Size::ZERO,
            max: self.max,
        }
    }

    /// Clamp a given size so that fits within the constraints.
    pub fn constrain(&self, size: impl Into<Size>) -> Size {
        size.into().clamp(self.min, self.max)
    }

    /// Returns the max size of these constraints.
    pub fn max(&self) -> Size {
        self.max
    }

    /// Returns the min size of these constraints.
    pub fn min(&self) -> Size {
        self.min
    }

    /// Whether there is an upper bound on the width.
    pub fn is_width_bounded(&self) -> bool {
        self.max.width.is_finite()
    }

    /// Whether there is an upper bound on the height.
    pub fn is_height_bounded(&self) -> bool {
        self.max.height.is_finite()
    }

    /// Check to see if these constraints are legit.
    ///
    /// Logs a warning if BoxConstraints are invalid.
    pub fn debug_check(&self, name: &str) {
        if !(0.0 <= self.min.width
            && self.min.width <= self.max.width
            && 0.0 <= self.min.height
            && self.min.height <= self.max.height)
        {
            log::warn!("Bad BoxConstraints passed to {}:", name);
            log::warn!("{:?}", self);
        }
    }

    /// Shrink min and max constraints by size
    pub fn shrink(&self, diff: impl Into<Size>) -> BoxConstraints {
        let diff = diff.into();
        let min = Size::new(
            (self.min().width - diff.width).max(0.),
            (self.min().height - diff.height).max(0.),
        );
        let max = Size::new(
            (self.max().width - diff.width).max(0.),
            (self.max().height - diff.height).max(0.),
        );

        BoxConstraints::new(min, max)
    }
}
