// Copyright 2020 The xi-editor Authors.
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

use float_cmp::ApproxEq;

use crate::kurbo::{Insets, Line, Point, Rect, Size, Vec2};

const SCALE_TARGET_DPI: f64 = 96.0;

/// Coordinate scaling between pixels and display points.
///
/// This holds the platform DPI and the equivalent scale factors.
///
/// To translate coordinates between pixels and display points you should use one of the
/// helper conversion methods of `Scale` or for manual conversion [`scale_x`] / [`scale_y`].
///
/// `Scale` is designed for responsive applications, including responding to platform DPI changes.
/// The platform DPI can change quickly, e.g. when moving a window from one monitor to another.
///
/// A copy of `Scale` will be stale as soon as the platform DPI changes.
///
/// [`scale_x`]: #method.scale_x
/// [`scale_y`]: #method.scale_y
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Scale {
    /// The platform reported DPI on the x axis.
    dpi_x: f64,
    /// The platform reported DPI on the y axis.
    dpi_y: f64,
    /// The scale factor on the x axis.
    scale_x: f64,
    /// The scale factor on the y axis.
    scale_y: f64,
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
/// the platform area pixel boundaries from the logcal area using a [`Scale`].
///
/// Even though the logcal area size can be fractional, the integer boundaries of that logical area
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
    fn to_px(&self, scale: &Scale) -> Self;

    /// Converts the scalable item from pixels into display points,
    /// using the x axis scale factor for coordinates on the x axis
    /// and the y axis scale factor for coordinates on the y axis.
    fn to_dp(&self, scale: &Scale) -> Self;
}

impl Default for Scale {
    fn default() -> Scale {
        Scale {
            dpi_x: SCALE_TARGET_DPI,
            dpi_y: SCALE_TARGET_DPI,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

impl Scale {
    /// Create a new `Scale` state based on the specified DPIs.
    ///
    /// Use this constructor if the platform provided DPI is the most accurate number.
    pub fn from_dpi(dpi_x: f64, dpi_y: f64) -> Scale {
        Scale {
            dpi_x,
            dpi_y,
            scale_x: dpi_x / SCALE_TARGET_DPI,
            scale_y: dpi_y / SCALE_TARGET_DPI,
        }
    }

    /// Create a new `Scale` state based on the specified scale factors.
    ///
    /// Use this constructor if the platform provided scale factor is the most accurate number.
    pub fn from_scale(scale_x: f64, scale_y: f64) -> Scale {
        Scale {
            dpi_x: SCALE_TARGET_DPI * scale_x,
            dpi_y: SCALE_TARGET_DPI * scale_y,
            scale_x,
            scale_y,
        }
    }

    /// Returns the x axis platform DPI associated with this `Scale`.
    #[inline]
    pub fn dpi_x(&self) -> f64 {
        self.dpi_x
    }

    /// Returns the y axis platform DPI associated with this `Scale`.
    #[inline]
    pub fn dpi_y(&self) -> f64 {
        self.dpi_y
    }

    /// Returns `true` if the specified DPI is approximately equal to the `Scale` DPI.
    #[inline]
    pub fn dpi_approx_eq(&self, dpi_x: f64, dpi_y: f64) -> bool {
        self.dpi_x.approx_eq(dpi_x, (f64::EPSILON, 2))
            && self.dpi_y.approx_eq(dpi_y, (f64::EPSILON, 2))
    }

    /// Returns the x axis scale factor.
    #[inline]
    pub fn scale_x(&self) -> f64 {
        self.scale_x
    }

    /// Returns the y axis scale factor.
    #[inline]
    pub fn scale_y(&self) -> f64 {
        self.scale_y
    }

    /// Converts from display points into pixels, using the x axis scale factor.
    #[inline]
    pub fn dp_to_px_x<T: Into<f64>>(&self, x: T) -> f64 {
        x.into() * self.scale_x
    }

    /// Converts from display points into pixels, using the y axis scale factor.
    #[inline]
    pub fn dp_to_px_y<T: Into<f64>>(&self, y: T) -> f64 {
        y.into() * self.scale_y
    }

    /// Converts from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    pub fn dp_to_px_xy<T: Into<f64>>(&self, x: T, y: T) -> (f64, f64) {
        (x.into() * self.scale_x, y.into() * self.scale_y)
    }

    /// Converts the `item` from display points into pixels,
    /// using the x axis scale factor for coordinates on the x axis
    /// and the y axis scale factor for coordinates on the y axis.
    #[inline]
    pub fn to_px<T: Scalable>(&self, item: &T) -> T {
        item.to_px(self)
    }

    /// Converts from pixels into display points, using the x axis scale factor.
    #[inline]
    pub fn px_to_dp_x<T: Into<f64>>(&self, x: T) -> f64 {
        x.into() / self.scale_x
    }

    /// Converts from pixels into display points, using the y axis scale factor.
    #[inline]
    pub fn px_to_dp_y<T: Into<f64>>(&self, y: T) -> f64 {
        y.into() / self.scale_y
    }

    /// Converts from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    pub fn px_to_dp_xy<T: Into<f64>>(&self, x: T, y: T) -> (f64, f64) {
        (x.into() / self.scale_x, y.into() / self.scale_y)
    }

    /// Converts the `item` from pixels into display points,
    /// using the x axis scale factor for coordinates on the x axis
    /// and the y axis scale factor for coordinates on the y axis.
    #[inline]
    pub fn to_dp<T: Scalable>(&self, item: &T) -> T {
        item.to_dp(self)
    }
}

impl Scalable for Vec2 {
    /// Converts a `Vec2` from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_px(&self, scale: &Scale) -> Vec2 {
        Vec2::new(self.x * scale.scale_x, self.y * scale.scale_y)
    }

    /// Converts a `Vec2` from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_dp(&self, scale: &Scale) -> Vec2 {
        Vec2::new(self.x / scale.scale_x, self.y / scale.scale_y)
    }
}

impl Scalable for Point {
    /// Converts a `Point` from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_px(&self, scale: &Scale) -> Point {
        Point::new(self.x * scale.scale_x, self.y * scale.scale_y)
    }

    /// Converts a `Point` from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_dp(&self, scale: &Scale) -> Point {
        Point::new(self.x / scale.scale_x, self.y / scale.scale_y)
    }
}

impl Scalable for Line {
    /// Converts a `Line` from display points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_px(&self, scale: &Scale) -> Line {
        Line::new(self.p0.to_px(scale), self.p1.to_px(scale))
    }

    /// Converts a `Line` from pixels into display points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    fn to_dp(&self, scale: &Scale) -> Line {
        Line::new(self.p0.to_dp(scale), self.p1.to_dp(scale))
    }
}

impl Scalable for Size {
    /// Converts a `Size` from display points into pixels,
    /// using the x axis scale factor for `width`
    /// and the y axis scale factor for `height`.
    #[inline]
    fn to_px(&self, scale: &Scale) -> Size {
        Size::new(self.width * scale.scale_x, self.height * scale.scale_y)
    }

    /// Converts a `Size` from pixels into points,
    /// using the x axis scale factor for `width`
    /// and the y axis scale factor for `height`.
    #[inline]
    fn to_dp(&self, scale: &Scale) -> Size {
        Size::new(self.width / scale.scale_x, self.height / scale.scale_y)
    }
}

impl Scalable for Rect {
    /// Converts a `Rect` from display points into pixels,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_px(&self, scale: &Scale) -> Rect {
        Rect::new(
            self.x0 * scale.scale_x,
            self.y0 * scale.scale_y,
            self.x1 * scale.scale_x,
            self.y1 * scale.scale_y,
        )
    }

    /// Converts a `Rect` from pixels into display points,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_dp(&self, scale: &Scale) -> Rect {
        Rect::new(
            self.x0 / scale.scale_x,
            self.y0 / scale.scale_y,
            self.x1 / scale.scale_x,
            self.y1 / scale.scale_y,
        )
    }
}

impl Scalable for Insets {
    /// Converts `Insets` from display points into pixels,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_px(&self, scale: &Scale) -> Insets {
        Insets::new(
            self.x0 * scale.scale_x,
            self.y0 * scale.scale_y,
            self.x1 * scale.scale_x,
            self.y1 * scale.scale_y,
        )
    }

    /// Converts `Insets` from pixels into display points,
    /// using the x axis scale factor for `x0` and `x1`
    /// and the y axis scale factor for `y0` and `y1`.
    #[inline]
    fn to_dp(&self, scale: &Scale) -> Insets {
        Insets::new(
            self.x0 / scale.scale_x,
            self.y0 / scale.scale_y,
            self.x1 / scale.scale_x,
            self.y1 / scale.scale_y,
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
    pub fn from_px<T: Into<Size>>(size: T, scale: &Scale) -> ScaledArea {
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
    pub fn from_dp<T: Into<Size>>(size: T, scale: &Scale) -> ScaledArea {
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
