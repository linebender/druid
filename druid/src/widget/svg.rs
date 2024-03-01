// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! An SVG widget.

use std::sync::Arc;

use resvg;
use usvg::Tree;

use crate::piet::{ImageBuf, ImageFormat, InterpolationMode};
use crate::widget::prelude::*;
use crate::{Rect, ScaledArea};

/// A widget that renders a SVG
pub struct Svg {
    tree: Arc<Tree>,
    default_size: Size,
    cached: Option<ImageBuf>,
}

impl Svg {
    /// Create an SVG-drawing widget from SvgData.
    ///
    /// The SVG will scale to fit its box constraints.
    pub fn new(tree: impl Into<Arc<Tree>>) -> Self {
        let tree = tree.into();
        Svg {
            default_size: Size::new(tree.size.width(), tree.size.height()),
            cached: None,
            tree,
        }
    }

    /// Rasterize the SVG into the specified size in pixels.
    fn render(&self, size_px: Size) -> Option<ImageBuf> {
        let fit = usvg::FitTo::Size(size_px.width as u32, size_px.height as u32);
        let mut pixmap =
            tiny_skia::Pixmap::new(size_px.width as u32, size_px.height as u32).unwrap();

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

        Some(ImageBuf::from_raw(
            pixmap.data(),
            ImageFormat::RgbaPremul,
            size_px.width as usize,
            size_px.height as usize,
        ))
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
        // preferred size comes from the svg
        let size = self.default_size;
        bc.constrain_aspect_ratio(size.height / size.width, size.width)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        let size = ctx.size();
        let area = ScaledArea::from_dp(size, ctx.scale());
        let size_px = area.size_px();

        let needs_render = self
            .cached
            .as_ref()
            .filter(|image_buf| image_buf.size() == size_px)
            .is_none();

        if needs_render {
            self.cached = self.render(size_px);
        }

        if self.cached.is_none() {
            tracing::error!("unable to paint SVG due to no rendered image");
            return;
        }

        let clip_rect = Rect::ZERO.with_size(size);
        let img = self.cached.as_ref().unwrap().to_image(ctx.render_ctx);
        ctx.clip(clip_rect);
        ctx.draw_image(&img, clip_rect, InterpolationMode::NearestNeighbor);
    }
}

/// Stored parsed SVG tree.
#[derive(Clone, Data)]
pub struct SvgData {
    tree: Arc<Tree>,
}

impl SvgData {
    /// Create a new SVG
    fn new(tree: Arc<Tree>) -> Self {
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

        match Tree::from_str(svg_str, &re_opt.to_ref()) {
            // TODO: Figure out if this needs to stay Arc, or if it can be switched to Rc
            #[allow(clippy::arc_with_non_send_sync)]
            Ok(tree) => Ok(SvgData::new(Arc::new(tree))),
            Err(err) => Err(err.into()),
        }
    }
}

impl From<SvgData> for Arc<Tree> {
    fn from(d: SvgData) -> Self {
        d.tree
    }
}

impl Default for SvgData {
    fn default() -> Self {
        SvgData::empty()
    }
}
