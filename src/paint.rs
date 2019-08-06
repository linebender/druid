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

//! Paint context and helper methods.

use std::borrow::{Borrow, Cow};
use std::ops::{Deref, DerefMut};

use crate::kurbo::{Point, Rect, Shape};

use crate::piet::{
    Color, FillRule, GradientStop, LinearGradient, Piet, RenderContext, StrokeStyle,
};

/// A context passed to paint methods of widgets.
///
/// Widgets paint their appearance by calling methods on the
/// `render_ctx`, which PaintCtx derefs to for convenience.
/// This struct is expected to grow, for example to include the
/// "damage region" indicating that only a subset of the entire
/// widget hierarchy needs repainting.
pub struct PaintCtx<'a, 'b: 'a> {
    /// The render context for actually painting.
    pub render_ctx: &'a mut Piet<'b>,
}

/// A brush that can be dependent on the geometry being drawn.
pub trait Brush: Sized {
    // Discussion point: instead of impl shape, should it be a closure
    // that computes bbox? This feels like it would be more general, and
    // would, for example, let us take the stroke width into account when
    // computing the bbox for stroke.
    fn make_brush<'a>(&'a self, piet: &mut Piet, shape: &impl Shape)
        -> Cow<'a, crate::piet::Brush>;
}

/// A flexible, ergonomic way to describe gradient stops.
// Discussion question: I think this could be done in piet as a
// simple `Into<Vec<GradientStop>>`.
pub trait GradientStops {
    fn to_vec(self) -> Vec<GradientStop>;
}

/// A "gradient description" that depends on the geometry being drawn
///
/// Right now, this is just a linear gradient, but we could either make
/// it an enum, or make a separate, similar type for radial gradients.
pub struct Gradient {
    start: UnitPoint,
    end: UnitPoint,
    stops: Vec<GradientStop>,
}

/// A representation of a point relative to a rectangle.
pub struct UnitPoint {
    u: f64,
    v: f64,
}

impl UnitPoint {
    pub const TOP_LEFT: UnitPoint = UnitPoint::new(0.0, 0.0);
    pub const TOP: UnitPoint = UnitPoint::new(0.5, 0.0);
    pub const TOP_RIGHT: UnitPoint = UnitPoint::new(1.0, 0.0);
    pub const LEFT: UnitPoint = UnitPoint::new(0.0, 0.5);
    pub const CENTER: UnitPoint = UnitPoint::new(0.5, 0.5);
    pub const RIGHT: UnitPoint = UnitPoint::new(1.0, 0.5);
    pub const BOTTOM_LEFT: UnitPoint = UnitPoint::new(0.0, 1.0);
    pub const BOTTOM: UnitPoint = UnitPoint::new(0.5, 1.0);
    pub const BOTTOM_RIGHT: UnitPoint = UnitPoint::new(1.0, 1.0);

    /// Create a new UnitPoint.
    ///
    /// The `u` and `v` coordinates describe the point, with 0.0 being
    /// left and top, and 1.0 being right and bottom, respectively.
    pub const fn new(u: f64, v: f64) -> UnitPoint {
        UnitPoint { u, v }
    }
    /// Given a rectangle, resolve the point within the rectangle.
    pub fn resolve(&self, rect: Rect) -> Point {
        Point::new(
            rect.x0 + self.u * (rect.x1 - rect.x0),
            rect.y0 + self.v * (rect.y1 - rect.y0),
        )
    }
}

impl Brush for Color {
    fn make_brush<'a>(
        &'a self,
        piet: &mut Piet,
        _shape: &impl Shape,
    ) -> Cow<'a, crate::piet::Brush> {
        Cow::Owned(piet.solid_brush(self.to_owned()))
    }
}

impl Brush for crate::piet::Brush {
    fn make_brush<'a>(
        &'a self,
        _piet: &mut Piet,
        _shape: &impl Shape,
    ) -> Cow<'a, crate::piet::Brush> {
        Cow::Borrowed(self)
    }
}

impl<'a, 'b: 'a> Deref for PaintCtx<'a, 'b> {
    type Target = Piet<'b>;

    fn deref(&self) -> &Self::Target {
        self.render_ctx
    }
}

impl<'a, 'b: 'a> DerefMut for PaintCtx<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.render_ctx
    }
}

// The block below expands the range of brush arguments to allow anything
// implementing the brush trait. This is a signifcant ergonomics improvement.
impl<'a, 'b> PaintCtx<'a, 'b> {
    /// Fill a shape.
    pub fn fill(&mut self, shape: impl Shape, brush: &impl Brush, fill_rule: FillRule) {
        let concrete_brush = brush.make_brush(self, &shape);
        self.render_ctx
            .fill(shape, concrete_brush.borrow(), fill_rule);
    }

    /// Stroke a shape.
    pub fn stroke(
        &mut self,
        shape: impl Shape,
        brush: &impl Brush,
        width: f64,
        style: Option<&StrokeStyle>,
    ) {
        let concrete_brush = brush.make_brush(self, &shape);
        self.render_ctx
            .stroke(shape, concrete_brush.borrow(), width, style);
    }

    // Note: we should make `draw_text` participate too, but it requires the bounding box
    // for gradient creation, and we don't have a good way to do that yet.
}

impl GradientStops for Vec<GradientStop> {
    fn to_vec(self) -> Vec<GradientStop> {
        self
    }
}

impl<'a> GradientStops for &'a [GradientStop] {
    fn to_vec(self) -> Vec<GradientStop> {
        self.to_owned()
    }
}

// Generate equally-spaced stops.
impl<'a> GradientStops for &'a [Color] {
    fn to_vec(self) -> Vec<GradientStop> {
        if self.is_empty() {
            Vec::new()
        } else {
            let denom = (self.len() - 1).max(1) as f32;
            self.iter()
                .enumerate()
                .map(|(i, c)| GradientStop {
                    pos: (i as f32) / denom,
                    color: c.to_owned(),
                })
                .collect()
        }
    }
}

impl<'a> GradientStops for (Color, Color) {
    fn to_vec(self) -> Vec<GradientStop> {
        let stops: &[Color] = &[self.0, self.1];
        GradientStops::to_vec(stops)
    }
}

impl<'a> GradientStops for (Color, Color, Color) {
    fn to_vec(self) -> Vec<GradientStop> {
        let stops: &[Color] = &[self.0, self.1, self.2];
        GradientStops::to_vec(stops)
    }
}

impl<'a> GradientStops for (Color, Color, Color, Color) {
    fn to_vec(self) -> Vec<GradientStop> {
        let stops: &[Color] = &[self.0, self.1, self.2, self.3];
        GradientStops::to_vec(stops)
    }
}

impl Gradient {
    pub fn new(start: UnitPoint, end: UnitPoint, stops: impl GradientStops) -> Gradient {
        Gradient {
            start,
            end,
            stops: stops.to_vec(),
        }
    }
}

impl Brush for Gradient {
    fn make_brush<'a>(
        &'a self,
        piet: &mut Piet,
        shape: &impl Shape,
    ) -> Cow<'a, crate::piet::Brush> {
        let rect = shape.bounding_box();
        let gradient = crate::piet::Gradient::Linear(LinearGradient {
            start: self.start.resolve(rect).to_vec2(),
            end: self.end.resolve(rect).to_vec2(),
            stops: self.stops.clone(),
        });
        // Perhaps the make_brush method should be fallible instead of panicking.
        Cow::Owned(piet.gradient(gradient).expect("error creating gradient"))
    }
}
