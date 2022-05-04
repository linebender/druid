// Copyright 2022 The Druid Authors.
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

use druid_shell::{
    kurbo::{Point, Size},
    piet::{Color, RenderContext, Text, TextLayoutBuilder},
};

use crate::{event::Event, id::IdPath};

use super::{LayoutCx, PaintCx, Widget};

#[derive(Default)]

pub struct Button {
    id_path: IdPath,
    label: String,
}

impl Button {
    pub fn new(id_path: &IdPath, label: String) -> Button {
        Button {
            id_path: id_path.clone(),
            label,
        }
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }
}

impl Widget for Button {
    fn event(&mut self, _event: &super::RawEvent, events: &mut Vec<Event>) {
        events.push(Event::new(self.id_path.clone(), ()));
    }

    fn layout(&mut self, _cx: &mut LayoutCx, _proposed_size: Size) -> Size {
        Size::new(100., 20.)
    }

    fn paint(&mut self, ctx: &mut PaintCx) {
        let layout = ctx
            .text()
            .new_text_layout(self.label.clone())
            .text_color(Color::WHITE)
            .build()
            .unwrap();
        ctx.draw_text(&layout, Point::ZERO);
    }
}
