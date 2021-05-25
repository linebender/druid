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

use crate::kurbo::{Point, Size};
use crate::piet::{
    Color, FontFamily, FontWeight, PietText, PietTextLayout, RenderContext, Text as _, TextLayout,
    TextLayoutBuilder,
};
use crate::{BoxConstraints, EventCtx, LayoutCtx, PaintCtx, Widget};

/// A widget that provides simple visual styling options to a child.
pub struct Text {
    text: String,
    size: f64,
    color: Color,
    font: FontFamily,
    weight: FontWeight,
    text_obj: Option<PietTextLayout>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Text {
            text: text.into(),
            size: 16.0,
            color: Color::grey(0.1),
            font: FontFamily::SYSTEM_UI,
            weight: FontWeight::NORMAL,
            text_obj: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.text_obj = None;
    }

    pub fn set_font_size(&mut self, size: f64) {
        self.size = size;
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_font(&mut self, font: FontFamily) {
        self.font = font;
    }

    pub fn set_weight(&mut self, weight: FontWeight) {
        self.weight = weight;
    }

    fn rebuild_text(&mut self, mut factory: PietText) {
        self.text_obj = factory
            .new_text_layout(self.text.clone())
            .font(self.font.clone(), self.size)
            .text_color(self.color.clone())
            .default_attribute(self.weight.clone())
            .build()
            .ok();
    }
}

impl Widget for Text {
    fn init(&mut self, ctx: &mut EventCtx) {
        self.rebuild_text(ctx.text())
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        if self.text_obj.is_none() {
            self.rebuild_text(ctx.text());
        }
        let text_size = self
            .text_obj
            .as_ref()
            .map(|obj| obj.size())
            .unwrap_or(Size::ZERO);
        bc.constrain(text_size)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        if let Some(obj) = self.text_obj.as_ref() {
            ctx.draw_text(obj, Point::ZERO)
        }
    }
}
