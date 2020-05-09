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

use crate::kurbo::{Point, Rect, Size};

const SCALE_TARGET_DPI: f64 = 96.0;

/// Resolution scaling state.
///
/// This holds the platform DPI, the platform area size in pixels,
/// the logical area size in display points, and the scale factors between them.
///
/// To translate coordinates between pixels and display points you should use one of the
/// helper conversion methods of `Scale` or for manual conversion [`scale_x`] / [`scale_y`].
///
/// The platform area size in pixels tends to be limited to integers and `Scale` works
/// under that assumption.
///  
/// The logical area size in display points is an unrounded conversion, which means that it is
/// often not limited to integers. This allows for accurate calculations of
/// the platform area pixel boundaries from the logcal area using the scale factors.
///
/// Even though the logcal area size can be fractional, the integer boundaries of that logical area
/// will still match up with the platform area pixel boundaries as often as the scale factor allows.
///
/// `Scale` is designed for responsive applications, including responding to platform DPI changes.
/// The platform DPI can change quickly, e.g. when moving a window from one monitor to another.
///
/// A clone of `Scale` will be stale as soon as the platform area size or the DPI changes.
///
/// [`scale_x`]: #method.scale_x
/// [`scale_y`]: #method.scale_y
// NOTE: Scale does not derive Copy because the data it contains can be short-lived.
//       Forcing an explicit clone can help remind people of this, and avoids accidental copies.
#[derive(Clone)]
pub struct Scale {
    /// The platform reported DPI on the x axis.
    dpi_x: f64,
    /// The platform reported DPI on the y axis.
    dpi_y: f64,
    /// The scale factor on the x axis.
    scale_x: f64,
    /// The scale factor on the y axis.
    scale_y: f64,
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
            size_dp: Size::ZERO,
            size_px: Size::ZERO,
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
            size_dp: Size::ZERO,
            size_px: Size::ZERO,
        }
    }

    /// Set the size of the scaled area in pixels.
    ///
    /// This updates the internal state and returns the same size in display points.
    pub fn set_size_px<T: Into<Size>>(&mut self, size: T) -> Size {
        let size = size.into();
        self.size_px = size;
        self.size_dp = self.to_dp(&size);
        self.size_dp
    }

    /// Set the size of the scaled area in pixels via conversion from display points.
    ///
    /// This updates the internal state and returns the same size in pixels.
    ///
    /// The calculated size in pixels is rounded away from zero to integers.
    /// That means that the scaled area size in display points isn't always the same
    /// as the `size` given to this method. To find out the new size in points use [`size_dp`].
    ///
    /// [`size_dp`]: #method.size_dp
    pub fn set_size_dp<T: Into<Size>>(&mut self, size: T) -> Size {
        let size = size.into();
        self.size_px = self.to_px(&size).expand();
        self.size_dp = self.to_dp(&self.size_px);
        self.size_px
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

impl std::fmt::Debug for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "DPI ({}, {}) => ({}, {}) | {:?} => {:?}",
            self.dpi_x, self.dpi_y, self.scale_x, self.scale_y, self.size_px, self.size_dp,
        )
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

// TODO: Replace usages of this with rect.expand() after kurbo#107 has landed.
#[allow(dead_code)]
#[doc(hidden)]
pub fn expand_rect(rect: Rect) -> Rect {
    let (x0, x1) = if rect.x0 < rect.x1 {
        (rect.x0.floor(), rect.x1.ceil())
    } else {
        (rect.x0.ceil(), rect.x1.floor())
    };
    let (y0, y1) = if rect.y0 < rect.y1 {
        (rect.y0.floor(), rect.y1.ceil())
    } else {
        (rect.y0.ceil(), rect.y1.floor())
    };
    Rect::new(x0, y0, x1, y1)
}
