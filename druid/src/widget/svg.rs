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

//! An SVG widget.

use std::error::Error;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use log::error;

use usvg;

use crate::{
    kurbo::BezPath,
    piet::{Color, RenderContext},
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point, Size,
    UpdateCtx, Widget,
};

/// A widget that renders a SVG
pub struct Svg<T> {
    // On construction the SVG string is parsed into a SvgData.
    svg_data: SvgData,
    phantom: PhantomData<T>,
}

impl<T: Data> Svg<T> {
    /// Create an SVG-drawing widget from SvgData.
    ///
    /// The SVG will scale to fit its box constraints.
    pub fn new(svg_data: SvgData) -> impl Widget<T> {
        Svg {
            svg_data,
            phantom: Default::default(),
        }
    }

    /// Measure the SVG's size
    #[allow(clippy::needless_return)]
    fn get_size(&self) -> Size {
        let root = self.svg_data.tree.root();
        match *root.borrow() {
            usvg::NodeKind::Svg(svg) => {
                return Size::new(svg.size.width(), svg.size.height());
            }
            _ => {
                //TODO: I don't think this is reachable?
                error!("This SVG has no size for some reason.");
                return Size::ZERO;
            }
        };
    }
}

impl<T: Data> Widget<T> for Svg<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}

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
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _data: &T, _env: &Env) {
        //TODO: options for aspect ratio or scaling based on height
        let scale = base_state.size().width / self.get_size().width;

        let origin_x = (base_state.size().width - (self.get_size().width * scale)) / 2.0;
        let origin_y = (base_state.size().height - (self.get_size().height * scale)) / 2.0;
        let origin = Point::new(origin_x, origin_y);

        self.svg_data.to_piet(scale, origin, paint_ctx);
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
    pub fn to_piet(&self, scale: f64, offset: Point, paint_ctx: &mut PaintCtx) {
        let root = self.tree.root();

        for n in root.children() {
            match *n.borrow() {
                usvg::NodeKind::Path(ref p) => {
                    let mut path = BezPath::new();
                    for segment in p.data.iter() {
                        match *segment {
                            usvg::PathSegment::MoveTo { x, y } => {
                                let x = (x * scale) + offset.x;
                                let y = (y * scale) + offset.y;
                                path.move_to((x, y));
                            }
                            usvg::PathSegment::LineTo { x, y } => {
                                let x = (x * scale) + offset.x;
                                let y = (y * scale) + offset.y;
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
                                let x1 = (x1 * scale) + offset.x;
                                let y1 = (y1 * scale) + offset.y;
                                let x2 = (x2 * scale) + offset.x;
                                let y2 = (y2 * scale) + offset.y;
                                let x = (x * scale) + offset.x;
                                let y = (y * scale) + offset.y;

                                path.curve_to((x1, y1), (x2, y2), (x, y));
                            }
                            usvg::PathSegment::ClosePath => {
                                path.close_path();
                            }
                        }
                    }
                    match &p.fill {
                        Some(fill) => {
                            let brush = color_from_usvg(&fill.paint, fill.opacity);
                            paint_ctx.fill(path.clone(), &brush);
                        }
                        None => {}
                    }

                    match &p.stroke {
                        Some(stroke) => {
                            let brush = color_from_usvg(&stroke.paint, stroke.opacity);
                            paint_ctx.stroke(path.clone(), &brush, stroke.width.value());
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
