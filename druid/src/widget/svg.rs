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

use crate::kurbo::{BezPath, Point, Size};
use crate::piet::{Color, RenderContext};

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};
use usvg::{Fill, NodeKind, Paint, PathSegment, Stroke};

use log::error;

use std::marker::PhantomData;

/// A widget that renders a SVG
pub struct SVG<T> {
    // On construction the SVG string is parsed into a usvg::Tree.
    tree: usvg::Tree,
    phantom: PhantomData<T>,
}

impl<T: Data> SVG<T> {
    /// Create an SVG-drawing widget from a valid SVG string literal.
    ///
    /// The SVG will scale to fit its box constraints.
    pub fn new_from_str(svg_str: &str) -> impl Widget<T> {
        let re_opt = usvg::Options {
            keep_named_groups: false,
            ..usvg::Options::default()
        };

        let tree = match usvg::Tree::from_str(svg_str, &re_opt) {
            Ok(tree) => tree,
            Err(err) => panic!("Couldn't parse SVG because: {}", err),
        };

        SVG {
            tree,
            phantom: Default::default(),
        }
    }

    /// Create an SVG-drawing widget from a valid path to an .svg file.
    ///
    /// The SVG will scale to fit its box constraints.
    pub fn new_from_path(path: &str) -> impl Widget<T> {
        let re_opt = usvg::Options {
            keep_named_groups: false,
            ..usvg::Options::default()
        };

        let tree = match usvg::Tree::from_file(path, &re_opt) {
            Ok(tree) => tree,
            Err(err) => panic!("Couldn't parse SVG because: {}", err),
        };

        SVG {
            tree,
            phantom: Default::default(),
        }
    }

    /// Measure the SVG's size
    fn get_size(&self) -> Size {
        let root = self.tree.root();
        match *root.borrow() {
            NodeKind::Svg(svg) => {
                return Size::new(svg.size.width(), svg.size.height());
            }
            _ => {
                //TODO: I don't think this is reachable?
                error!("This SVG has no size for some reason.");
                return Size::ZERO;
            }
        };
    }

    /// Convert a parsed usvg tree into Piet draw instructions
    fn svg_to_piet(&self, scale: f64, offset: Point, paint_ctx: &mut PaintCtx) {
        let root = self.tree.root();

        for n in root.children() {
            match *n.borrow() {
                NodeKind::Path(ref p) => {
                    let mut path = BezPath::new();
                    for segment in p.data.iter() {
                        match *segment {
                            PathSegment::MoveTo { x, y } => {
                                let x = (x * scale) + offset.x;
                                let y = (y * scale) + offset.y;
                                path.move_to((x, y));
                            }
                            PathSegment::LineTo { x, y } => {
                                let x = (x * scale) + offset.x;
                                let y = (y * scale) + offset.y;
                                path.line_to((x, y));
                            }
                            PathSegment::CurveTo {
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

                                //QUESTION is this order correct?
                                path.curve_to((x1, y1), (x2, y2), (x, y));
                            }
                            PathSegment::ClosePath => {
                                path.close_path();
                            }
                        }
                    }
                    match &p.fill {
                        Some(fill) => {
                            let brush = color_from_fill(fill);
                            paint_ctx.fill(path.clone(), &brush);
                        }
                        None => {}
                    }

                    match &p.stroke {
                        Some(stroke) => {
                            let brush = color_from_stroke(stroke);
                            paint_ctx.stroke(path.clone(), &brush, stroke.width.value());
                        }
                        None => {}
                    }
                }
                NodeKind::Defs => {
                    // TODO: what is this?
                }
                _ => {
                    // TODO: handle more of the SVG spec.
                    error!("{:?} is unimplemented", n.clone());
                }
            }
        }
    }
}

impl<T: Data> Widget<T> for SVG<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _data: &T, _env: &Env) {
        //TODO: options for aspect ratio or scaling based on height
        let scale = base_state.size().width / self.get_size().width;

        let origin_x = (base_state.size().width - (self.get_size().width * scale)) / 2.0;
        let origin_y = (base_state.size().height - (self.get_size().height * scale)) / 2.0;
        let origin = Point::new(origin_x, origin_y);

        self.svg_to_piet(scale, origin, paint_ctx);
    }

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

    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}
}

fn color_from_fill(fill: &Fill) -> Color {
    match fill.paint {
        Paint::Color(c) => {
            let alpha = (fill.opacity.value() * 255.0) as u8;
            Color::rgba8(c.red, c.green, c.blue, alpha)
        }
        _ => {
            //TODO: figure this out!
            error!("I don't know what a Paint::Link is so here's some pink.");
            Color::rgb8(255, 192, 203)
        }
    }
}

fn color_from_stroke(stroke: &Stroke) -> Color {
    match stroke.paint {
        Paint::Color(c) => {
            let alpha = (stroke.opacity.value() * 255.0) as u8;
            Color::rgba8(c.red, c.green, c.blue, alpha)
        }
        _ => {
            //TODO: figure this out!
            error!("I don't know what a Paint::Link is so here's some pink.");
            Color::rgb8(255, 192, 203)
        }
    }
}
