// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use crate::{Affine, Data, Size};

// These are based on https://api.flutter.dev/flutter/painting/BoxFit.html
/// Strategies for inscribing a rectangle inside another rectangle.
#[derive(Default, Clone, Data, Copy, PartialEq, Eq)]
pub enum FillStrat {
    /// As large as possible without changing aspect ratio of image and all of image shown
    #[default]
    Contain,
    /// As large as possible with no dead space so that some of the image may be clipped
    Cover,
    /// Fill the widget with no dead space, aspect ratio of widget is used
    Fill,
    /// Fill the height with the images aspect ratio, some of the image may be clipped
    FitHeight,
    /// Fill the width with the images aspect ratio, some of the image may be clipped
    FitWidth,
    /// Do not scale
    None,
    /// Scale down to fit but do not scale up
    ScaleDown,
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
