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

use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;

use log::error;

use crate::{
    kurbo::BezPath, widget::common::FillStrat, Affine, BoxConstraints, Color, Data, Env, Event,
    EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Rect, RenderContext, Size, UpdateCtx,
    Widget,
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

    /// Measure the SVG's size
    #[allow(clippy::needless_return)]
    fn get_size(&self) -> Size {
        let root = self.svg_data.tree.root();
        match *root.borrow() {
            usvg::NodeKind::Svg(svg) => {
                // Borrow checker gets confused without an explicit return
                return Size::new(svg.size.width(), svg.size.height());
            }
            _ => {
                //TODO: I don't think this is reachable?
                error!("This SVG has no size for some reason.");
                return Size::ZERO;
            }
        };
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
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &T, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.debug_check("SVG");

        if bc.is_width_bounded() {
            bc.max()
        } else {
            bc.constrain(self.get_size())
        }
    }
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let offset_matrix = self.fill.affine_to_fill(ctx.size(), self.get_size());

        let clip_rect = Rect::ZERO.with_size(ctx.size());

        // The SvgData's to_piet function dose not clip to the svg's size
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
        let root = self.tree.root();
        for n in root.children() {
            match *n.borrow() {
                usvg::NodeKind::Path(ref p) => {
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

                    path.apply_affine(Affine::new([
                        p.transform.a,
                        p.transform.b,
                        p.transform.c,
                        p.transform.d,
                        p.transform.e,
                        p.transform.f,
                    ]));
                    path.apply_affine(offset_matrix);

                    match &p.fill {
                        Some(fill) => {
                            let brush = color_from_usvg(&fill.paint, fill.opacity);
                            ctx.fill(path.clone(), &brush);
                        }
                        None => {}
                    }

                    match &p.stroke {
                        Some(stroke) => {
                            let brush = color_from_usvg(&stroke.paint, stroke.opacity);
                            ctx.stroke(path.clone(), &brush, stroke.width.value());
                        }
                        None => {}
                    }
                }
                usvg::NodeKind::Defs => {
                    // TODO: implement defs
                }
                _ => {
                    // TODO: handle more of the SVG spec.
                    error!("{:?} is unimplemented", n.clone());
                }
            }
        }
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

fn color_from_usvg(paint: &usvg::Paint, opacity: usvg::Opacity) -> Color {
    match paint {
        usvg::Paint::Color(c) => Color::rgb8(c.red, c.green, c.blue).with_alpha(opacity.value()),
        _ => {
            //TODO: implement link
            error!("We don't support Paint::Link yet, so here's some pink.");
            Color::rgb8(255, 192, 203)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

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
