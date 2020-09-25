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

//! Resolution scale related helpers.

use crate::kurbo::{Insets, Line, Point, Rect, Size, Vec2};

/// Coordinate scaling between pixels and display points.
///
/// This holds the platform scale factors.
///
/// ## Pixels and Display Points
///
/// A pixel (**px**) represents the smallest controllable area of color on the platform.
/// A display point (**dp**) is a resolution independent logical unit.
/// When developing your application you should primarily be thinking in display points.
/// These display points will be automatically converted into pixels under the hood.
/// One pixel is equal to one display point when the platform scale factor is `1.0`.
///
/// Read more about pixels and display points [in the druid book].
///
/// ## Converting with `Scale`
///
/// To translate coordinates between pixels and display points you should use one of the
/// helper conversion methods of `Scale` or for manual conversion [`x`] / [`y`].
///
/// `Scale` is designed for responsive applications, including responding to platform scale changes.
/// The platform scale can change quickly, e.g. when moving a window from one monitor to another.
///
/// A copy of `Scale` will be stale as soon as the platform scale changes.
///
/// [`x`]: #method.x
/// [`y`]: #method.y
/// [in the druid book]: https://linebender.org/druid/resolution_independence.html
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Scale {
    /// The scale factor on the x axis.
    x: f64,
    /// The scale factor on the y axis.
    y: f64,
}

/// A specific area scaling state.
///
/// This holds the platform area size in pixels and the logical area size in display points.
///
/// The platform area size in pixels tends to be limited to integers and `ScaledArea` works
/// under that assumption.
///
/// The logical area size in display points is an unrounded conversion, which means that it is
/// often not limited to integers. This allows for accurate calculations of
/// the platform area pixel boundaries from the logical area using a [`Scale`].
///
/// Even though the logical area size can be fractional, the integer boundaries of that logical area
/// will still match up with the platform area pixel boundaries as often as the scale factor allows.
///
/// A copy of `ScaledArea` will be stale as soon as the platform area size changes.
///
/// [`Scale`]: struct.Scale.html
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ScaledArea {
    /// The size of the scaled area in display points.
    size_dp: Size,
    /// The size of the scaled area in pixels.
    size_px: Size,
}

/// The `Scalable` trait describes how coordinates should be translated
/// from display points into pixels and vice versa using a [`Scale`].
///
/// [`Scale`]: struct.Scale.html
pub trait Scalable {
    /// Converts the scalable item from display points into pixels,
    /// using the x axis scale factor for coordinates on the x axis
    /// and the y axis scale factor for coordinates on the y axis.
    fn to_px(&self, scale: Scale) -> Self;

    /// Converts the scalable item from pixels into display points,
    /// using the x axis scale factor for coordinates on the x axis
    /// and the y axis scale factor for coordinates on the y axis.
    fn to_dp(&self, scale: Scale) -> Self;
}

impl Default for Scale {
    fn default() -> Scale {
        Scale { x: 1.0, y: 1.0 }
    }
}

impl Scale {
    /// Create a new `Scale` based on the specified axis factors.
    pub fn new(x: f64, y: f64) -> Scale {
        Scale { x, y }
    }

    /// Returns the x axis scale factor.
    #[inline]
    pub fn x(self) -> f64 {
        self.x
    }

    /// Returns the y axis scale factor.
    #[inline]
    pub fn y(self) -> f64 {
        self.y
    }

    /// Converts from pixels into display points, using the x axis scale factor.
    #[inline]
    pub fn px_to_dp_x<T: Into<f64>>(self, x: T) -> f64 {
        x.into() / self.x
    }

    /// Converts from pixels into display points, using the y axis scale factor.
    #[inline]
    pub fn px_to_dp_y<T: Into<f64>>(self, y: T) -> f64 {
        y.into() / self.y
    }

    /// Converts from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    pub fn px_to_dp_xy<T: Into<f64>>(self, x: T, y: T) -> (f64, f64) {
        (x.into() / self.x, y.into() / self.y)
    }
}

impl Scalable for Vec2 {
    /// Converts a `Vec2` from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_px(&self, scale: Scale) -> Vec2 {
        Vec2::new(self.x * scale.x, self.y * scale.y)
    }

    /// Converts a `Vec2` from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_dp(&self, scale: Scale) -> Vec2 {
        Vec2::new(self.x / scale.x, self.y / scale.y)
    }
}

impl Scalable for Point {
    /// Converts a `Point` from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_px(&self, scale: Scale) -> Point {
        Point::new(self.x * scale.x, self.y * scale.y)
    }

    /// Converts a `Point` from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_dp(&self, scale: Scale) -> Point {
        Point::new(self.x / scale.x, self.y / scale.y)
    }
}

impl Scalable for Line {
    /// Converts a `Line` from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_px(&self, scale: Scale) -> Line {
        Line::new(self.p0.to_px(scale), self.p1.to_px(scale))
    }

    /// Converts a `Line` from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_dp(&self, scale: Scale) -> Line {
        Line::new(self.p0.to_dp(scale), self.p1.to_dp(scale))
    }
}

impl Scalable for Size {
    /// Converts a `Size` from display points into pixels,
    /// using the x axis scale factor for `width`
    /// and the y axis scale factor for `height`.
    #[inline]
    fn to_px(&self, scale: Scale) -> Size {
        Size::new(self.width * scale.x, self.height * scale.y)
    }

    /// Converts a `Size` from pixels into points,
    /// using the x axis scale factor for `width`
    /// and the y axis scale factor for `height`.
    #[inline]
    fn to_dp(&self, scale: Scale) -> Size {
        Size::new(self.width / scale.x, self.height / scale.y)
    }
}

impl Scalable for Rect {
    /// Converts a `Rect` from display points into pixels,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_px(&self, scale: Scale) -> Rect {
        Rect::new(
            self.x0 * scale.x,
            self.y0 * scale.y,
            self.x1 * scale.x,
            self.y1 * scale.y,
        )
    }

    /// Converts a `Rect` from pixels into display points,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_dp(&self, scale: Scale) -> Rect {
        Rect::new(
            self.x0 / scale.x,
            self.y0 / scale.y,
            self.x1 / scale.x,
            self.y1 / scale.y,
        )
    }
}

impl Scalable for Insets {
    /// Converts `Insets` from display points into pixels,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_px(&self, scale: Scale) -> Insets {
        Insets::new(
            self.x0 * scale.x,
            self.y0 * scale.y,
            self.x1 * scale.x,
            self.y1 * scale.y,
        )
    }

    /// Converts `Insets` from pixels into display points,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_dp(&self, scale: Scale) -> Insets {
        Insets::new(
            self.x0 / scale.x,
            self.y0 / scale.y,
            self.x1 / scale.x,
            self.y1 / scale.y,
        )
    }
}

impl Default for ScaledArea {
    fn default() -> ScaledArea {
        ScaledArea {
            size_dp: Size::ZERO,
            size_px: Size::ZERO,
        }
    }
}

impl ScaledArea {
    /// Create a new scaled area from pixels.
    pub fn from_px<T: Into<Size>>(size: T, scale: Scale) -> ScaledArea {
        let size_px = size.into();
        let size_dp = size_px.to_dp(scale);
        ScaledArea { size_dp, size_px }
    }

    /// Create a new scaled area from display points.
    ///
    /// The calculated size in pixels is rounded away from zero to integers.
    /// That means that the scaled area size in display points isn't always the same
    /// as the `size` given to this function. To find out the new size in points use [`size_dp`].
    ///
    /// [`size_dp`]: #method.size_dp
    pub fn from_dp<T: Into<Size>>(size: T, scale: Scale) -> ScaledArea {
        let size_px = size.into().to_px(scale).expand();
        let size_dp = size_px.to_dp(scale);
        ScaledArea { size_dp, size_px }
    }

    /// Returns the scaled area size in display points.
    #[inline]
    pub fn size_dp(&self) -> Size {
        self.size_dp
    }

    /// Returns the scaled area size in pixels.
    #[inline]
    pub fn size_px(&self) -> Size {
        self.size_px
    }
}
