// Copyright 2018 Google LLC
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

use direct2d::RenderTarget;
use direct2d::brush::SolidColorBrush;
use directwrite::{self, TextFormat, TextLayout};

use xi_win_shell::util::default_text_options;
use xi_win_shell::window::{MouseButton, MouseType};

use {BoxConstraints, Geometry, LayoutResult};
use {HandlerCtx, Id, LayoutCtx, ListenerCtx, PaintCtx};
use widget::Widget;

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

    pub fn bind(self, ctx: &mut ListenerCtx) -> Id {
        ctx.add(self, &[])
    }

    fn get_layout(&self, dwrite_factory: &directwrite::Factory) -> TextLayout {
        // TODO: caching of both the format and the layout
        let format = TextFormat::create(&dwrite_factory)
            .with_family("Segoe UI")
            .with_size(15.0)
            .build()
            .unwrap();
        let layout = TextLayout::create(&dwrite_factory)
            .with_text(&self.label)
            .with_font(&format)
            .with_width(1e6)
            .with_height(1e6)
            .build().unwrap();
        layout
    }
}

impl Widget for Label {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let text_layout = self.get_layout(paint_ctx.dwrite_factory());
        let rt = paint_ctx.render_target();
        let fg = SolidColorBrush::create(rt).with_color(0xf0f0ea).build().unwrap();
        let (x, y) = geom.pos;
        rt.draw_text_layout((x, y), &text_layout, &fg, default_text_options());
    }

    fn layout(&mut self, bc: &BoxConstraints, _children: &[Id], _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx) -> LayoutResult
    {
        // TODO: measure text properly
        LayoutResult::Size(bc.constrain((100.0, 17.0)))
    }

    fn mouse(&mut self, x: f32, y: f32, mods: u32, which: MouseButton, ty: MouseType,
        ctx: &mut HandlerCtx) -> bool
    {
        println!("button {} {} {:x} {:?} {:?}", x, y, mods, which, ty);
        if ty == MouseType::Down {
            ctx.send_event(true);
        }
        true
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

    pub fn bind(self, ctx: &mut ListenerCtx) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for Button {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        {
            let rt = paint_ctx.render_target();
            let bg = SolidColorBrush::create(rt).with_color(0x404048).build().unwrap();
            rt.fill_rectangle(
                (geom.pos.0, geom.pos.1, geom.pos.0 + geom.size.0, geom.pos.1 + geom.size.1),
                &bg);
        }
        self.label.paint(paint_ctx, geom);
    }

    fn layout(&mut self, bc: &BoxConstraints, children: &[Id], size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx) -> LayoutResult
    {
        self.label.layout(bc, children, size, ctx)
    }

    fn mouse(&mut self, x: f32, y: f32, mods: u32, which: MouseButton, ty: MouseType,
        ctx: &mut HandlerCtx) -> bool
    {
        println!("button {} {} {:x} {:?} {:?}", x, y, mods, which, ty);
        if ty == MouseType::Down {
            ctx.send_event(true);
        }
        true
    }

    fn poke(&mut self, payload: &mut Any, ctx: &mut HandlerCtx) -> bool {
        self.label.poke(payload, ctx)
    }
}
