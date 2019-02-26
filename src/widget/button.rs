// Copyright 2018 The xi-editor Authors.
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

//! A button widget

use std::any::Any;

use kurbo::Rect;
use piet::{FillRule, FontBuilder, RenderContext, Text, TextLayoutBuilder};
use piet_common::Piet;

use crate::widget::Widget;
use crate::{BoxConstraints, Geometry, LayoutResult};
use crate::{HandlerCtx, Id, LayoutCtx, MouseEvent, PaintCtx, Ui};

/// A text label with no interaction.
pub struct Label {
    label: String,
}

/// A clickable button with a label.
pub struct Button {
    label: Label,
}

impl Label {
    pub fn new<S: Into<String>>(label: S) -> Label {
        Label {
            label: label.into(),
        }
    }

    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }

    fn get_layout(&self, rt: &mut Piet, font_size: f32) -> <Piet as RenderContext>::TextLayout {
        // TODO: caching of both the format and the layout
        let font = rt
            .text()
            .new_font_by_name("Segoe UI", font_size)
            .unwrap()
            .build()
            .unwrap();
        rt.text()
            .new_text_layout(&font, &self.label)
            .unwrap()
            .build()
            .unwrap()
    }
}

impl Widget for Label {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let font_size = 15.0;
        let text_layout = self.get_layout(paint_ctx.render_ctx, font_size);
        let brush = paint_ctx.render_ctx.solid_brush(0xf0f0eaff).unwrap();

        let pos = (geom.pos.0, geom.pos.1 + font_size);
        paint_ctx
            .render_ctx
            .draw_text(&text_layout, pos, &brush);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        // TODO: measure text properly
        LayoutResult::Size(bc.constrain((100.0, 17.0)))
    }

    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        if let Some(string) = payload.downcast_ref::<String>() {
            self.label = string.clone();
            ctx.invalidate();
            true
        } else {
            println!("downcast failed");
            false
        }
    }
}

impl Button {
    pub fn new<S: Into<String>>(label: S) -> Button {
        Button {
            label: Label::new(label),
        }
    }

    pub fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for Button {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        {
            let is_active = paint_ctx.is_active();
            let is_hot = paint_ctx.is_hot();
            let bg_color = match (is_active, is_hot) {
                (true, true) => 0x606068ff,
                (false, true) => 0x505058ff,
                _ => 0x404048ff,
            };
            let brush = paint_ctx.render_ctx.solid_brush(bg_color).unwrap();
            let (x, y) = geom.pos;
            let (width, height) = geom.size;
            let rect = Rect::new(
                x as f64,
                y as f64,
                x as f64 + width as f64,
                y as f64 + height as f64,
            );
            paint_ctx
                .render_ctx
                .fill(rect, &brush, FillRule::NonZero);
        }
        self.label.paint(paint_ctx, geom);
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        self.label.layout(bc, children, size, ctx)
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        if event.count > 0 {
            ctx.set_active(true);
        } else {
            ctx.set_active(false);
            if ctx.is_hot() {
                ctx.send_event(true);
            }
        }
        ctx.invalidate();
        true
    }

    fn on_hot_changed(&mut self, _hot: bool, ctx: &mut HandlerCtx) {
        ctx.invalidate();
    }

    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        self.label.poke(payload, ctx)
    }
}
