// Copyright 2019 The Druid Authors.
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

//! An SVG widget.

use std::{collections::HashMap, error::Error, rc::Rc, str::FromStr, sync::Arc};
use tracing::{instrument, trace};

use crate::{
    kurbo::BezPath,
    piet::{self, FixedLinearGradient, GradientStop, LineCap, LineJoin, StrokeStyle},
    widget::common::FillStrat,
    widget::prelude::*,
    Affine, Color, Data, Point, Rect,
};

/// A widget that renders a SVG
pub struct Svg {
    svg_data: SvgData,
    fill: FillStrat,
}

impl Svg {
    /// Create an SVG-drawing widget from SvgData.
    ///
    /// The SVG will scale to fit its box constraints.
    pub fn new(svg_data: SvgData) -> Self {
        Svg {
            svg_data,
            fill: FillStrat::default(),
        }
    }

    /// A builder-style method for specifying the fill strategy.
    pub fn fill_mode(mut self, mode: FillStrat) -> Self {
        self.fill = mode;
        self
    }

    /// Modify the widget's `FillStrat`.
    pub fn set_fill_mode(&mut self, newfil: FillStrat) {
        self.fill = newfil;
    }
}

impl<T: Data> Widget<T> for Svg {
    #[instrument(name = "Svg", level = "trace", skip(self, _ctx, _event, _data, _env))]
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    #[instrument(name = "Svg", level = "trace", skip(self, _ctx, _event, _data, _env))]
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &T, _env: &Env) {}

    #[instrument(
        name = "Svg",
        level = "trace",
        skip(self, _ctx, _old_data, _data, _env)
    )]
    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    #[instrument(
        name = "Svg",
        level = "trace",
        skip(self, _layout_ctx, bc, _data, _env)
    )]
    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.debug_check("SVG");
        // preferred size comes from the svg
        let size = self.svg_data.size();
        let constrained_size = bc.constrain_aspect_ratio(size.height / size.width, size.width);
        trace!("Computed size: {}", constrained_size);
        constrained_size
    }

    #[instrument(name = "Svg", level = "trace", skip(self, ctx, _data, _env))]
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let offset_matrix = self.fill.affine_to_fill(ctx.size(), self.svg_data.size());

        let clip_rect = Rect::ZERO.with_size(ctx.size());

        // The SvgData's to_piet function does not clip to the svg's size
        // CairoRenderContext is very like druids but with some extra goodies like clip
        ctx.clip(clip_rect);
        self.svg_data.to_piet(offset_matrix, ctx);
    }
}

/// Stored SVG data.
/// Implements `FromStr` and can be converted to piet draw instructions.
#[derive(Clone)]
pub struct SvgData {
    tree: Arc<usvg::Tree>,
}

impl SvgData {
    /// Create an empty SVG
    pub fn empty() -> Self {
        let re_opt = usvg::Options {
            keep_named_groups: false,
            ..usvg::Options::default()
        };

        let empty_svg = r###"
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
              <g fill="none">
              </g>
          </svg>
        "###;

        SvgData {
            tree: Arc::new(usvg::Tree::from_str(empty_svg, &re_opt).unwrap()),
        }
    }

    /// Convert SvgData into Piet draw instructions
    pub fn to_piet(&self, offset_matrix: Affine, ctx: &mut PaintCtx) {
        let mut state = SvgRenderer::new(offset_matrix * self.inner_affine());
        // I actually made `SvgRenderer` able to handle a stack of `<defs>`, but I'm gonna see if
        // resvg always puts them at the top.
        let root = self.tree.root();
        for n in root.children() {
            state.render_node(&n, ctx);
        }
    }

    /// Calculates the transform that should be applied first to the svg path data, to convert from
    /// image coordinates to piet coordinates.
    fn inner_affine(&self) -> Affine {
        let viewbox = self.viewbox();
        let size = self.size();
        // we want to move the viewbox top left to (0,0) and then scale it from viewbox size to
        // size.
        // TODO respect preserveAspectRatio
        let t = Affine::translate((viewbox.min_x(), viewbox.min_y()));
        let scale =
            Affine::scale_non_uniform(size.width / viewbox.width(), size.height / viewbox.height());
        scale * t
    }

    /// Get the viewbox for the svg. This is the area that should be drawn.
    fn viewbox(&self) -> Rect {
        let root = self.tree.root();
        let rect = match *root.borrow() {
            usvg::NodeKind::Svg(svg) => {
                let r = svg.view_box.rect;
                Rect::new(r.left(), r.top(), r.right(), r.bottom())
            }
            _ => {
                tracing::error!(
                    "this SVG has no viewbox. It is expected that usvg always adds a viewbox"
                );
                Rect::ZERO
            }
        };
        rect
    }

    /// Get the size of the svg. This is the size that the svg requests to be drawn. If it is
    /// different from the viewbox size, then scaling will be required.
    fn size(&self) -> Size {
        let root = self.tree.root();
        let rect = match *root.borrow() {
            usvg::NodeKind::Svg(svg) => {
                let s = svg.size;
                Size::new(s.width(), s.height())
            }
            _ => {
                tracing::error!(
                    "this SVG has no size. It is expected that usvg always adds a size"
                );
                Size::ZERO
            }
        };
        rect
    }
}

impl Default for SvgData {
    fn default() -> Self {
        SvgData::empty()
    }
}

impl FromStr for SvgData {
    type Err = Box<dyn Error>;

    fn from_str(svg_str: &str) -> Result<Self, Self::Err> {
        let re_opt = usvg::Options {
            keep_named_groups: false,
            ..usvg::Options::default()
        };

        match usvg::Tree::from_str(svg_str, &re_opt) {
            Ok(tree) => Ok(SvgData {
                tree: Arc::new(tree),
            }),
            Err(err) => Err(err.into()),
        }
    }
}

struct SvgRenderer {
    offset_matrix: Affine,
    defs: Defs,
}

impl SvgRenderer {
    fn new(offset_matrix: Affine) -> Self {
        Self {
            offset_matrix,
            defs: Defs::new(),
        }
    }

    /// Take a usvg node and render it to the given context.
    fn render_node(&mut self, n: &usvg::Node, ctx: &mut PaintCtx) {
        match *n.borrow() {
            usvg::NodeKind::Path(ref p) => self.render_path(p, ctx),
            usvg::NodeKind::Defs => {
                // children are defs
                for def in n.children() {
                    match &*def.borrow() {
                        usvg::NodeKind::LinearGradient(linear_gradient) => {
                            self.linear_gradient_def(linear_gradient, ctx);
                        }
                        other => tracing::error!("unsupported element: {:?}", other),
                    }
                }
            }
            usvg::NodeKind::Group(_) => {
                // TODO I'm not sure if we need to apply the transform, or if usvg has already
                // done it for us? I'm guessing the latter for now, but that could easily be wrong.
                for child in n.children() {
                    self.render_node(&child, ctx);
                }
            }
            _ => {
                // TODO: handle more of the SVG spec.
                tracing::error!("{:?} is unimplemented", n.clone());
            }
        }
    }

    /// Take a usvg path and render it to the given context.
    fn render_path(&self, p: &usvg::Path, ctx: &mut PaintCtx) {
        if matches!(
            p.visibility,
            usvg::Visibility::Hidden | usvg::Visibility::Collapse
        ) {
            // skip rendering
            return;
        }

        let mut path = BezPath::new();
        for segment in p.data.iter() {
            match *segment {
                usvg::PathSegment::MoveTo { x, y } => {
                    path.move_to((x, y));
                }
                usvg::PathSegment::LineTo { x, y } => {
                    path.line_to((x, y));
                }
                usvg::PathSegment::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    path.curve_to((x1, y1), (x2, y2), (x, y));
                }
                usvg::PathSegment::ClosePath => {
                    path.close_path();
                }
            }
        }

        path.apply_affine(self.offset_matrix * transform_to_affine(p.transform));

        match &p.fill {
            Some(fill) => {
                let brush = self.brush_from_usvg(&fill.paint, fill.opacity, ctx);
                if let usvg::FillRule::EvenOdd = fill.rule {
                    ctx.fill_even_odd(path.clone(), &*brush);
                } else {
                    ctx.fill(path.clone(), &*brush);
                }
            }
            None => {}
        }

        match &p.stroke {
            Some(stroke) => {
                let brush = self.brush_from_usvg(&stroke.paint, stroke.opacity, ctx);
                let mut stroke_style = StrokeStyle::new()
                    .line_join(match stroke.linejoin {
                        usvg::LineJoin::Miter => LineJoin::Miter,
                        usvg::LineJoin::Round => LineJoin::Round,
                        usvg::LineJoin::Bevel => LineJoin::Bevel,
                    })
                    .line_cap(match stroke.linecap {
                        usvg::LineCap::Butt => LineCap::Butt,
                        usvg::LineCap::Round => LineCap::Round,
                        usvg::LineCap::Square => LineCap::Square,
                    })
                    .miter_limit(stroke.miterlimit.value());
                if let Some(dash_array) = &stroke.dasharray {
                    stroke_style.set_dash(dash_array.clone(), stroke.dashoffset as f64);
                }
                ctx.stroke_styled(path, &*brush, stroke.width.value(), &stroke_style);
            }
            None => {}
        }
    }

    fn linear_gradient_def(&mut self, lg: &usvg::LinearGradient, ctx: &mut PaintCtx) {
        // Get start and stop of gradient and transform them to image space (TODO check we need to
        // apply offset matrix)
        let start = self.offset_matrix * Point::new(lg.x1, lg.y1);
        let end = self.offset_matrix * Point::new(lg.x2, lg.y2);
        let stops: Vec<_> = lg
            .base
            .stops
            .iter()
            .map(|stop| GradientStop {
                pos: stop.offset.value() as f32,
                color: color_from_svg(stop.color, stop.opacity),
            })
            .collect();

        // TODO error handling
        let gradient = FixedLinearGradient { start, end, stops };
        trace!("gradient: {} => {:?}", lg.id, gradient);
        let gradient = ctx.gradient(gradient).unwrap();
        self.defs.add_def(lg.id.clone(), gradient);
    }

    fn brush_from_usvg(
        &self,
        paint: &usvg::Paint,
        opacity: usvg::Opacity,
        ctx: &mut PaintCtx,
    ) -> Rc<piet::Brush> {
        match paint {
            usvg::Paint::Color(c) => {
                // TODO I'm going to assume here that not retaining colors is OK.
                let color = color_from_svg(*c, opacity);
                Rc::new(ctx.solid_brush(color))
            }
            usvg::Paint::Link(id) => self.defs.find(id).unwrap(),
        }
    }
}

// TODO just support linear gradient for now.
type Def = piet::Brush;

/// A map from id to <def>
struct Defs(HashMap<String, Rc<Def>>);

impl Defs {
    fn new() -> Self {
        Defs(HashMap::new())
    }

    /// Add a def.
    fn add_def(&mut self, id: String, def: Def) {
        self.0.insert(id, Rc::new(def));
    }

    /// Look for a def by id.
    fn find(&self, id: &str) -> Option<Rc<Def>> {
        self.0.get(id).cloned()
    }
}

/// Convert a usvg transform to a kurbo `Affine`.
fn transform_to_affine(t: usvg::Transform) -> Affine {
    Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])
}

fn color_from_svg(c: usvg::Color, opacity: usvg::Opacity) -> Color {
    Color::rgb8(c.red, c.green, c.blue).with_alpha(opacity.value())
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use test_env_log::test;

    #[test]
    fn usvg_transform_vs_affine() {
        assert_eq!(
            transform_to_affine(usvg::Transform::new_translate(1., 2.)),
            Affine::translate((1., 2.))
        );
        assert_eq!(
            transform_to_affine(usvg::Transform::new_scale(1., 2.)),
            Affine::scale_non_uniform(1., 2.)
        );
        // amazingly we get actual equality here
        assert_eq!(
            transform_to_affine(usvg::Transform::new_rotate(180.)),
            Affine::rotate(std::f64::consts::PI)
        );
    }

    #[test]
    fn translate() {
        use crate::tests::harness::Harness;

        let svg_data = SvgData::from_str(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2 2'>
        <g>
            <g>
                <rect width='1' height='1'/>
            </g>
        </g>
        <g transform=\"translate(1, 1)\">
            <g>
                <rect width='1' height='1'/>
            </g>
        </g>
    </svg>",
        )
        .unwrap();

        let svg_widget = Svg::new(svg_data);

        Harness::create_with_render(
            true,
            svg_widget,
            Size::new(400., 600.),
            |harness| {
                harness.send_initial_events();
                harness.just_layout();
                harness.paint();
            },
            |target| {
                let raw_pixels = target.into_raw();
                assert_eq!(raw_pixels.len(), 400 * 600 * 4);

                // Being a tall widget with a square image the top and bottom rows will be
                // the padding color and the middle rows will not have any padding.

                // Check that the middle row 400 pix wide is 200 black then 200 white.
                let expecting: Vec<u8> = [
                    vec![41, 41, 41, 255].repeat(200),
                    vec![0, 0, 0, 255].repeat(200),
                ]
                .concat();
                assert_eq!(raw_pixels[400 * 300 * 4..400 * 301 * 4], expecting[..]);

                // Check that all of the last 100 rows are all the background color.
                let expecting: Vec<u8> = vec![41, 41, 41, 255].repeat(400 * 100);
                assert_eq!(
                    raw_pixels[400 * 600 * 4 - 4 * 400 * 100..400 * 600 * 4],
                    expecting[..]
                );
            },
        )
    }

    #[test]
    fn scale() {
        use crate::tests::harness::Harness;

        let svg_data = SvgData::from_str(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2 2'>
        <g>
            <g>
                <rect width='1' height='1'/>
            </g>
        </g>
        <g transform=\"translate(1, 1)\">
            <g transform=\"scale(1, 2)\">
                <rect width='1' height='0.5'/>
            </g>
        </g>
    </svg>",
        )
        .unwrap();

        let svg_widget = Svg::new(svg_data);

        Harness::create_with_render(
            true,
            svg_widget,
            Size::new(400., 600.),
            |harness| {
                harness.send_initial_events();
                harness.just_layout();
                harness.paint();
            },
            |target| {
                let raw_pixels = target.into_raw();
                assert_eq!(raw_pixels.len(), 400 * 600 * 4);

                // Being a tall widget with a square image the top and bottom rows will be
                // the padding color and the middle rows will not have any padding.

                // Check that the middle row 400 pix wide is 200 black then 200 white.
                let expecting: Vec<u8> = [
                    vec![41, 41, 41, 255].repeat(200),
                    vec![0, 0, 0, 255].repeat(200),
                ]
                .concat();
                assert_eq!(raw_pixels[400 * 300 * 4..400 * 301 * 4], expecting[..]);

                // Check that all of the last 100 rows are all the background color.
                let expecting: Vec<u8> = vec![41, 41, 41, 255].repeat(400 * 100);
                assert_eq!(
                    raw_pixels[400 * 600 * 4 - 4 * 400 * 100..400 * 600 * 4],
                    expecting[..]
                );
            },
        )
    }
}
