// Copyright 2020 The Druid Authors.
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

use crate::{Affine, Size};

// These are based on https://api.flutter.dev/flutter/painting/BoxFit-class.html
/// Strategies for inscribing a rectangle inside another rectangle.
#[derive(Clone, Copy, PartialEq)]
pub enum FillStrat {
    /// As large as posible without changing aspect ratio of image and all of image shown
    Contain,
    /// As large as posible with no dead space so that some of the image may be clipped
    Cover,
    /// Fill the widget with no dead space, aspect ratio of widget is used
    Fill,
    /// Fill the hight with the images aspect ratio, some of the image may be clipped
    FitHeight,
    /// Fill the width with the images aspect ratio, some of the image may be clipped
    FitWidth,
    /// Do not scale
    None,
    /// Scale down to fit but do not scale up
    ScaleDown,
}

impl Default for FillStrat {
    fn default() -> Self {
        FillStrat::Contain
    }
}

impl FillStrat {
    /// Calculate an origin and scale for an image with a given `FillStrat`.
    ///
    /// This takes some properties of a widget and a fill strategy and returns an affine matrix
    /// used to position and scale the image in the widget.
    pub fn affine_to_fill(self, parent: Size, fit_box: Size) -> Affine {
        let raw_scalex = parent.width / fit_box.width;
        let raw_scaley = parent.height / fit_box.height;

        let (scalex, scaley) = match self {
            FillStrat::Contain => {
                let scale = raw_scalex.min(raw_scaley);
                (scale, scale)
            }
            FillStrat::Cover => {
                let scale = raw_scalex.max(raw_scaley);
                (scale, scale)
            }
            FillStrat::Fill => (raw_scalex, raw_scaley),
            FillStrat::FitHeight => (raw_scaley, raw_scaley),
            FillStrat::FitWidth => (raw_scalex, raw_scalex),
            FillStrat::ScaleDown => {
                let scale = raw_scalex.min(raw_scaley).min(1.0);
                (scale, scale)
            }
            FillStrat::None => (1.0, 1.0),
        };

        let origin_x = (parent.width - (fit_box.width * scalex)) / 2.0;
        let origin_y = (parent.height - (fit_box.height * scaley)) / 2.0;

        Affine::new([scalex, 0., 0., scaley, origin_x, origin_y])
    }
}
