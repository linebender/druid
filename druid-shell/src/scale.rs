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
/// the logical area size in points, and the scale factors between them.
///
/// To translate coordinates between pixels and points you should use one of the
/// helper conversion methods of `Scale` or for manual conversion [`scale_x`] / [`scale_y`].
///
/// The platform area size in pixels tends to be limited to integers and `Scale` works
/// under that assumption.
///  
/// The logical area size in points is an unrounded conversion, which means that it is
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
    /// The size of the scaled area in points.
    size_pt: Size,
    /// The size of the scaled area in pixels.
    size_px: Size,
}

impl Scale {
    /// Create a new `Scale` state based on the specified DPI.
    pub fn new(dpi_x: f64, dpi_y: f64) -> Scale {
        Scale {
            dpi_x,
            dpi_y,
            scale_x: dpi_x / SCALE_TARGET_DPI,
            scale_y: dpi_y / SCALE_TARGET_DPI,
            size_pt: Size::ZERO,
            size_px: Size::ZERO,
        }
    }

    /// Set the size of the scaled area in pixels.
    ///
    /// This updates the internal state and returns the same size in points.
    pub fn set_size_px<T: Into<Size>>(&mut self, size: T) -> Size {
        let size = size.into();
        self.size_px = size;
        self.size_pt = self.px_to_pt_size(size);
        self.size_pt
    }

    /// Set the size of the scaled area in pixels via conversion from points.
    ///
    /// This updates the internal state and returns the same size in pixels.
    ///
    /// The calculated size in pixels is rounded away from zero to integers.
    /// That means that the scaled area size in points isn't always the same
    /// as the `size` given to this method. To find out the new size in points use [`size_pt`].
    ///
    /// [`size_pt`]: #method.size_pt
    pub fn set_size_pt<T: Into<Size>>(&mut self, size: T) -> Size {
        let size = size.into();
        self.size_px = self.pt_to_px_size(size).expand();
        self.size_pt = self.px_to_pt_size(self.size_px);
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

    /// Returns the scaled area size in points.
    #[inline]
    pub fn size_pt(&self) -> Size {
        self.size_pt
    }

    /// Returns the scaled area size in pixels.
    #[inline]
    pub fn size_px(&self) -> Size {
        self.size_px
    }

    /// Converts from points into pixels, using the x axis scale factor.
    #[inline]
    pub fn pt_to_px_x<T: Into<f64>>(&self, x: T) -> f64 {
        x.into() * self.scale_x
    }

    /// Converts from points into pixels, using the y axis scale factor.
    #[inline]
    pub fn pt_to_px_y<T: Into<f64>>(&self, y: T) -> f64 {
        y.into() * self.scale_y
    }

    /// Converts from points into pixels,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    pub fn pt_to_px_xy<T: Into<f64>>(&self, x: T, y: T) -> (f64, f64) {
        (x.into() * self.scale_x, y.into() * self.scale_y)
    }

    /// Converts a `Point` from points into pixels,
    /// using the x axis scale factor for `Point::x` and the y axis scale factor for `Point::y`.
    #[inline]
    pub fn pt_to_px_point<T: Into<Point>>(&self, point: T) -> Point {
        let point = point.into();
        Point::new(point.x * self.scale_x, point.y * self.scale_y)
    }

    /// Converts a `Rect` from points into pixels,
    /// using the x axis scale factor for `Rect::x0` and `Rect::x1`
    /// and the y axis scale factor for `Rect::y0` and `Rect::y1`.
    #[inline]
    pub fn pt_to_px_rect<T: Into<Rect>>(&self, rect: T) -> Rect {
        let rect = rect.into();
        Rect::new(
            rect.x0 * self.scale_x,
            rect.y0 * self.scale_y,
            rect.x1 * self.scale_x,
            rect.y1 * self.scale_y,
        )
    }

    /// Converts a `Size` from points into pixels,
    /// using the x axis scale factor for `Size::width`
    /// and the y axis scale factor for `Size::height`.
    #[inline]
    pub fn pt_to_px_size<T: Into<Size>>(&self, size: T) -> Size {
        let size = size.into();
        Size::new(size.width * self.scale_x, size.height * self.scale_y)
    }

    /// Converts from pixels into points, using the x axis scale factor.
    #[inline]
    pub fn px_to_pt_x<T: Into<f64>>(&self, x: T) -> f64 {
        x.into() / self.scale_x
    }

    /// Converts from pixels into points, using the y axis scale factor.
    #[inline]
    pub fn px_to_pt_y<T: Into<f64>>(&self, y: T) -> f64 {
        y.into() / self.scale_y
    }

    /// Converts from pixels into points,
    /// using the x axis scale factor for `x` and the y axis scale factor for `y`.
    #[inline]
    pub fn px_to_pt_xy<T: Into<f64>>(&self, x: T, y: T) -> (f64, f64) {
        (x.into() / self.scale_x, y.into() / self.scale_y)
    }

    /// Converts a `Point` from pixels into points,
    /// using the x axis scale factor for `Point::x` and the y axis scale factor for `Point::y`.
    #[inline]
    pub fn px_to_pt_point<T: Into<Point>>(&self, point: T) -> Point {
        let point = point.into();
        Point::new(point.x / self.scale_x, point.y / self.scale_y)
    }

    /// Converts a `Rect` from pixels into points,
    /// using the x axis scale factor for `Rect::x0` and `Rect::x1`
    /// and the y axis scale factor for `Rect::y0` and `Rect::y1`.
    #[inline]
    pub fn px_to_pt_rect<T: Into<Rect>>(&self, rect: T) -> Rect {
        let rect = rect.into();
        Rect::new(
            rect.x0 / self.scale_x,
            rect.y0 / self.scale_y,
            rect.x1 / self.scale_x,
            rect.y1 / self.scale_y,
        )
    }

    /// Converts a `Size` from pixels into points,
    /// using the x axis scale factor for `Size::width`
    /// and the y axis scale factor for `Size::height`.
    #[inline]
    pub fn px_to_pt_size<T: Into<Size>>(&self, size: T) -> Size {
        let size = size.into();
        Size::new(size.width / self.scale_x, size.height / self.scale_y)
    }
}

impl std::fmt::Debug for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "DPI ({}, {}) => ({}, {}) | {:?} => {:?}",
            self.dpi_x, self.dpi_y, self.scale_x, self.scale_y, self.size_px, self.size_pt,
        )
    }
}

// TODO: Replace usages of this with rect.expand() after kurbo#107 has landed.
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
