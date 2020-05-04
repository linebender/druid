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
#[derive(Clone)]
pub struct Scale {
    /// The platform reported DPI.
    dpi: f64,
    /// The ideal scaling factor based on the DPI.
    scale: f64,
    /// The actual current scale factor on the x axis.
    scale_x: f64,
    /// The actual current scale factor on the y axis.
    scale_y: f64,
    /// The size of the scaled area in points.
    size_pt: Size,
    /// The size of the scaled area in pixels.
    size_px: Size,
}

impl Scale {
    /// Create a new `Scale` state based on the specified `dpi`.
    pub fn new(dpi: f64) -> Scale {
        let scale = dpi / SCALE_TARGET_DPI;
        Scale {
            dpi,
            scale,
            scale_x: scale,
            scale_y: scale,
            size_pt: Size::ZERO,
            size_px: Size::ZERO,
        }
    }

    /// Set the size of the scaled area in points.
    ///
    /// This updates the internal scaling state and returns the same size in pixels.
    ///
    /// The calculated size in pixels is rounded to integers.
    pub fn set_size_pt(&mut self, size: Size) -> Size {
        self.size_px = (size * self.scale).round();
        self.size_pt = size;
        self.scale_x = self.size_px.width / self.size_pt.width;
        self.scale_y = self.size_px.height / self.size_pt.height;
        self.size_px
    }

    /// Set the size of the scaled area in pixels.
    ///
    /// This updates the internal scaling state and returns the same size in points.
    ///
    /// The calculated size in points is rounded to integers.
    pub fn set_size_px(&mut self, size: Size) -> Size {
        self.size_pt = div_size(size, self.scale).round();
        self.size_px = size;
        self.scale_x = self.size_px.width / self.size_pt.width;
        self.scale_y = self.size_px.height / self.size_pt.height;
        self.size_pt
    }

    /// Returns the platform DPI associated with this `Scale`.
    #[inline]
    pub fn dpi(&self) -> f64 {
        self.dpi
    }

    /// Returns `true` if the specified `dpi` is approximately equal to the `Scale` dpi.
    #[inline]
    pub fn dpi_approx_eq(&self, dpi: f64) -> bool {
        self.dpi.approx_eq(dpi, (f64::EPSILON, 2))
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
            "DPI {} => {} => ({}, {}) | {:?} => {:?}",
            self.dpi, self.scale, self.scale_x, self.scale_y, self.size_pt, self.size_px
        )
    }
}

// TODO: Remove this if kurbo::Size gets division support
#[inline]
fn div_size(size: Size, rhs: f64) -> Size {
    Size::new(size.width / rhs, size.height / rhs)
}
