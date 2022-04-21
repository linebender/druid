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
    kurbo::{Insets, Size},
    piet::{
        Color, LinearGradient, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
        UnitPoint,
    },
};

use crate::{event::Event, id::IdPath, VertAlignment};

use super::{
    align::{FirstBaseline, LastBaseline, SingleAlignment},
    contexts::LifeCycleCx,
    AlignCx, EventCx, LayoutCx, LifeCycle, PaintCx, RawEvent, UpdateCx, Widget,
};

#[derive(Default)]

pub struct Label {
    // id_path: IdPath,
    label: String,
    layout: Option<PietTextLayout>,
}

impl Label {
    // pub fn new(id_path: &IdPath, label: String) -> Label {
    pub fn new(label: String) -> Label {
        Label {
            // id_path: id_path.clone(),
            label,
            layout: None
        }
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }
}

impl Widget for Label {
    fn update(&mut self, cx: &mut UpdateCx) {
        cx.request_layout()
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {}

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        match event {
            LifeCycle::HotChanged(_) => cx.request_paint(),
            _ => (),
        }
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        let min_height = 24.0;
        let layout = cx
            .text()
            .new_text_layout(self.label.clone())
            .text_color(Color::WHITE)
            .build()
            .unwrap();
        let size = Size::new(
            layout.size().width,
            (layout.size().height).max(min_height),
        );
        self.layout = Some(layout);
        (Size::new(10.0, min_height), size)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        let size = Size::new(
            proposed_size
                .width
                .clamp(cx.min_size().width, cx.max_size().width),
            cx.max_size().height,
        );
        size
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let layout = self.layout.as_ref().unwrap();
        let offset = (cx.size().to_vec2() - layout.size().to_vec2()) * 0.5;
        cx.draw_text(layout, offset.to_point());
    }
}
