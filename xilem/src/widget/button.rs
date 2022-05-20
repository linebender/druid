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

use super::{
    contexts::LifeCycleCx, EventCx, LayoutCx, LifeCycle, PaintCx, RawEvent, UpdateCx, Widget,
};

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

const FIXED_SIZE: Size = Size::new(100., 20.);

impl Widget for Button {
    fn update(&mut self, _cx: &mut UpdateCx) {
        // TODO: probably want to request layout when string changes
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        match event {
            RawEvent::MouseDown(_) => cx.add_event(Event::new(self.id_path.clone(), ())),
            _ => (),
        };
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        match event {
            LifeCycle::HotChanged(_) => cx.request_paint(),
        }
    }

    fn prelayout(&mut self, _cx: &mut LayoutCx) -> (Size, Size) {
        // TODO: do text layout here.
        (FIXED_SIZE, FIXED_SIZE)
    }

    fn layout(&mut self, _cx: &mut LayoutCx, _proposed_size: Size) -> Size {
        FIXED_SIZE
    }

    // TODO: alignment

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
