// Copyright 2021 The Druid Authors.
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

//! A widget that draws text.

use super::layout::LayoutHost;
use super::Text;
use crate::kurbo::{Insets, Point, Size};
use crate::piet::{Color, RenderContext};
use crate::{BoxConstraints, EventCtx, LayoutCtx, MouseEvent, PaintCtx, Widget};

/// A widget that provides simple visual styling options to a child.
pub struct Button {
    text: LayoutHost<Text>,
    on_click: Option<Box<dyn FnMut()>>,
    hovered: bool,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Button {
            text: LayoutHost::new(Text::new(text)),
            on_click: None,
            hovered: false,
        }
    }

    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl Widget for Button {
    fn init(&mut self, ctx: &mut EventCtx) {
        self.text.init(ctx);
    }

    fn mouse_move(&mut self, ctx: &mut EventCtx, _event: &MouseEvent) {
        if ctx.hovered() != self.hovered {
            ctx.request_paint();
            self.hovered = ctx.hovered();
        }
    }

    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        if event.button.is_left() && event.count == 1 {
            ctx.set_mouse_focus(true);
            ctx.request_paint();
        }
    }

    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        if event.button.is_left() && ctx.mouse_focused() {
            ctx.request_paint();
            ctx.set_mouse_focus(false);
            if ctx.hovered() {
                if let Some(f) = self.on_click.as_mut() {
                    f()
                }
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        const INSETS: Insets = Insets::uniform_xy(4.0, 2.0);
        let text_size = self.text.layout(ctx, bc);
        let self_size = text_size + INSETS.size();
        self.text.set_origin(Point::new(INSETS.x0, INSETS.y0));
        bc.constrain(self_size)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let rect = ctx.frame().to_rounded_rect(2.0);
        if ctx.hovered() || ctx.mouse_focused() {
            ctx.fill(rect, &Color::GRAY);
        } else {
            ctx.fill(rect, &Color::WHITE);
        }
        if ctx.mouse_focused() {
            ctx.stroke(rect, &Color::BLACK, 2.0);
        } else {
            ctx.stroke(rect, &Color::GRAY, 2.0);
        }
        self.text.paint(ctx);
    }
}
