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

use druid;
use druid::RenderContext;
use resvg;

#[allow(dead_code)]
pub fn new(data: impl Into<std::sync::Arc<usvg::Tree>>) -> Svg {
    Svg::new(data.into())
}

#[allow(dead_code)]
pub fn from_str(s: &str) -> Result<SvgData, <SvgData as std::str::FromStr>::Err> {
    use std::str::FromStr;
    SvgData::from_str(s)
}

/// A widget that renders a SVG
pub struct Svg {
    tree: std::sync::Arc<usvg::Tree>,
    default_size: druid::Size,
    cached: Option<(druid::Size, druid::piet::ImageBuf)>,
}

impl Svg {
    /// Create an SVG-drawing widget from SvgData.
    ///
    /// The SVG will scale to fit its box constraints.
    pub fn new(tree: impl Into<std::sync::Arc<usvg::Tree>>) -> Self {
        let tree = tree.into();
        Svg {
            default_size: druid::Size::new(tree.size.width(), tree.size.height()),
            cached: None::<(druid::Size, druid::piet::ImageBuf)>,
            tree,
        }
    }

    fn render(&self, size: druid::Size) -> Option<druid::piet::ImageBuf> {
        let fit = usvg::FitTo::Size(size.width as u32, size.height as u32);
        let mut pixmap = tiny_skia::Pixmap::new(size.width as u32, size.height as u32).unwrap();

        if resvg::render(
            &self.tree,
            fit,
            tiny_skia::Transform::identity(),
            pixmap.as_mut(),
        )
        .is_none()
        {
            tracing::error!("unable to render svg");
            return None;
        }

        Some(druid::piet::ImageBuf::from_raw(
            pixmap.data(),
            druid::piet::ImageFormat::RgbaPremul,
            size.width as usize,
            size.height as usize,
        ))
    }
}

impl<T: druid::Data> druid::Widget<T> for Svg {
    fn event(
        &mut self,
        _ctx: &mut druid::EventCtx,
        _event: &druid::Event,
        _data: &mut T,
        _env: &druid::Env,
    ) {
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut druid::LifeCycleCtx,
        _event: &druid::LifeCycle,
        _data: &T,
        _env: &druid::Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut druid::UpdateCtx, _old_data: &T, _data: &T, _env: &druid::Env) {
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        _data: &T,
        _env: &druid::Env,
    ) -> druid::Size {
        // preferred size comes from the svg
        let size = self.default_size;
        bc.constrain_aspect_ratio(size.height / size.width, size.width)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, _data: &T, _env: &druid::Env) {
        let size = ctx.size();

        let cached = self.cached.as_ref().filter(|(csize, _)| *csize == size);
        let cached = match cached {
            Some(current) => Some(current.clone()),
            None => self.render(size).map(|i| (size, i)),
        };
        let cached = match cached {
            Some(current) => current,
            None => {
                tracing::error!("unable to paint svg");
                return;
            }
        };

        let clip_rect = druid::Rect::ZERO.with_size(cached.0);
        let img = cached.1.to_image(ctx.render_ctx);
        ctx.clip(clip_rect);
        ctx.draw_image(
            &img,
            clip_rect,
            druid::piet::InterpolationMode::NearestNeighbor,
        );
        self.cached = Some(cached);
    }
}

/// Stored parsed SVG tree.
#[derive(Clone, druid::Data)]
pub struct SvgData {
    tree: std::sync::Arc<usvg::Tree>,
}

impl SvgData {
    fn new(tree: std::sync::Arc<usvg::Tree>) -> Self {
        Self { tree }
    }

    /// Create an empty SVG
    pub fn empty() -> Self {
        use std::str::FromStr;

        let empty_svg = r###"
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                <g fill="none">
                </g>
            </svg>
        "###;

        SvgData::from_str(empty_svg).unwrap()
    }
}

impl std::str::FromStr for SvgData {
    type Err = Box<dyn std::error::Error>;

    fn from_str(svg_str: &str) -> Result<Self, Self::Err> {
        let re_opt = usvg::Options {
            keep_named_groups: false,
            ..usvg::Options::default()
        };

        match usvg::Tree::from_str(svg_str, &re_opt) {
            Ok(tree) => Ok(SvgData::new(std::sync::Arc::new(tree))),
            Err(err) => Err(err.into()),
        }
    }
}

impl From<SvgData> for std::sync::Arc<usvg::Tree> {
    fn from(d: SvgData) -> Self {
        d.tree
    }
}

impl Default for SvgData {
    fn default() -> Self {
        SvgData::empty()
    }
}
