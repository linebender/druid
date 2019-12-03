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

use crate::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Size, UpdateCtx,
    Widget,
};

use log::error;
use std::marker::PhantomData;

#[cfg(feature = "svg")]
const EMPTY_SVG: &str = r###"
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 0 0">
      <g fill="none">
      </g>
  </svg>
"###;

#[cfg(feature = "svg")]
use crate::{
    kurbo::BezPath,
    piet::{Color, RenderContext},
    Point,
};

#[cfg(feature = "svg")]
use usvg;

/// A widget that renders a SVG
#[cfg(feature = "svg")]
pub struct Svg<T> {
    // On construction the SVG string is parsed into a usvg::Tree.
    tree: usvg::Tree,
    phantom: PhantomData<T>,
}

#[cfg(feature = "svg")]
impl<T: Data> Svg<T> {
    /// Create an SVG-drawing widget from a valid SVG string literal.
    ///
    /// The SVG will scale to fit its box constraints.
    /// If SVG is invalid a blank SVG will be rendered instead.
    pub fn new_from_str(svg_str: &str) -> impl Widget<T> {
        Svg {
            tree: svg_tree_from_str(svg_str),
            phantom: Default::default(),
        }
    }

    /// Create an SVG-drawing widget from a valid path to an .svg file.
    ///
    /// The SVG will scale to fit its box constraints.
    /// If SVG is missing or invalid a blank SVG will be rendered instead.
    pub fn new_from_path(path: impl AsRef<std::path::Path>) -> impl Widget<T> {
        Svg {
            tree: svg_tree_from_path(path),
            phantom: Default::default(),
        }
    }

    /// Measure the SVG's size
    #[allow(clippy::needless_return)]
    fn get_size(&self) -> Size {
        let root = self.tree.root();
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

#[cfg(feature = "svg")]
impl<T: Data> Widget<T> for Svg<T> {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _data: &T, _env: &Env) {
        //TODO: options for aspect ratio or scaling based on height
        let scale = base_state.size().width / self.get_size().width;

        let origin_x = (base_state.size().width - (self.get_size().width * scale)) / 2.0;
        let origin_y = (base_state.size().height - (self.get_size().height * scale)) / 2.0;
        let origin = Point::new(origin_x, origin_y);

        svg_to_piet(&self.tree, scale, origin, paint_ctx);
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

// Create a raw usvg tree from an SVG string.
// Pass this tree to `svg_to_piet()` to draw it.
#[cfg(feature = "svg")]
pub fn svg_tree_from_str(svg_str: &str) -> usvg::Tree {
    let re_opt = usvg::Options {
        keep_named_groups: false,
        ..usvg::Options::default()
    };

    match usvg::Tree::from_str(svg_str, &re_opt) {
        Ok(tree) => tree,
        Err(err) => {
            error!("{}", err);
            usvg::Tree::from_str(EMPTY_SVG, &re_opt).unwrap()
        }
    }
}

// Create a raw usvg tree from a path to an SVG.
// Pass this tree to `svg_to_piet()` to draw it.
#[cfg(feature = "svg")]
pub fn svg_tree_from_path(path: impl AsRef<std::path::Path>) -> usvg::Tree {
    let re_opt = usvg::Options {
        keep_named_groups: false,
        ..usvg::Options::default()
    };

    match usvg::Tree::from_file(path, &re_opt) {
        Ok(tree) => tree,
        Err(err) => {
            error!("{}", err);
            usvg::Tree::from_str(EMPTY_SVG, &re_opt).unwrap()
        }
    }
}

/// Convert a parsed usvg tree into Piet draw instructions
#[cfg(feature = "svg")]
pub fn svg_to_piet(tree: &usvg::Tree, scale: f64, offset: Point, paint_ctx: &mut PaintCtx) {
    let root = tree.root();

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

#[cfg(feature = "svg")]
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

/// A fake SVG widget to notify users of the "svg" feature flag.
#[cfg(not(feature = "svg"))]
#[derive(Default)]
pub struct FakeSvg<T: Data> {
    phantom: PhantomData<T>,
}

#[cfg(not(feature = "svg"))]
impl<T: Data> FakeSvg<T> {
    /// A fake SVG widget to notify users of the "svg" feature flag.
    pub fn new() -> impl Widget<T> {
        error!("The SVG widget requires druid's \"svg\" feature flag");
        FakeSvg {
            phantom: Default::default(),
        }
    }
    /// A fake SVG widget to notify users of the "svg" feature flag.
    pub fn new_from_str(_svg_str: &str) -> impl Widget<T> {
        error!("The SVG widget requires druid's \"svg\" feature flag");
        FakeSvg {
            phantom: Default::default(),
        }
    }
    /// A fake SVG widget to notify users of the "svg" feature flag.
    pub fn new_from_path(_path: impl AsRef<std::path::Path>) -> impl Widget<T> {
        error!("The SVG widget requires druid's \"svg\" feature flag");
        FakeSvg {
            phantom: Default::default(),
        }
    }
}

#[cfg(not(feature = "svg"))]
impl<T: Data> Widget<T> for FakeSvg<T> {
    fn paint(&mut self, _paint_ctx: &mut PaintCtx, _base_state: &BaseState, _data: &T, _env: &Env) {
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}
}
