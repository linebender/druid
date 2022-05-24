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

pub struct Button {
    id_path: IdPath,
    label: String,
    layout: Option<PietTextLayout>,
}

impl Button {
    pub fn new(id_path: &IdPath, label: String) -> Button {
        Button {
            id_path: id_path.clone(),
            label,
            layout: None,
        }
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.layout = None;
    }
}

// See druid's button for info.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

impl Widget for Button {
    fn update(&mut self, cx: &mut UpdateCx) {
        cx.request_layout();
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        match event {
            RawEvent::MouseDown(_) => {
                cx.set_active(true);
                // TODO: request paint
            }
            RawEvent::MouseUp(_) => {
                if cx.is_hot() {
                    cx.add_event(Event::new(self.id_path.clone(), ()));
                }
                cx.set_active(false);
                // TODO: request paint
            }
            _ => (),
        };
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        match event {
            LifeCycle::HotChanged(_) => cx.request_paint(),
            _ => (),
        }
    }

    fn measure(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let min_height = 24.0;
        let layout = cx
            .text()
            .new_text_layout(self.label.clone())
            .text_color(Color::rgb8(0xf0, 0xf0, 0xea))
            .build()
            .unwrap();
        let size = Size::new(
            layout.size().width + padding.width,
            (layout.size().height + padding.height).max(min_height),
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

    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        if alignment.id() == FirstBaseline.id() || alignment.id() == LastBaseline.id() {
            let layout = self.layout.as_ref().unwrap();
            if let Some(metric) = layout.line_metric(0) {
                let value = 0.5 * (cx.size().height - layout.size().height) + metric.baseline;
                cx.aggregate(alignment, value);
            }
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let is_hot = cx.is_hot();
        let is_active = cx.is_active();
        let button_border_width = 2.0;
        let rounded_rect = cx
            .size()
            .to_rect()
            .inset(-0.5 * button_border_width)
            .to_rounded_rect(4.0);
        let border_color = if is_hot {
            Color::rgb8(0xa1, 0xa1, 0xa1)
        } else {
            Color::rgb8(0x3a, 0x3a, 0x3a)
        };
        let bg_gradient = if is_active {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (Color::rgb8(0x3a, 0x3a, 0x3a), Color::rgb8(0xa1, 0xa1, 0xa1)),
            )
        } else {
            LinearGradient::new(
                UnitPoint::TOP,
                UnitPoint::BOTTOM,
                (Color::rgb8(0xa1, 0xa1, 0xa1), Color::rgb8(0x3a, 0x3a, 0x3a)),
            )
        };
        cx.stroke(rounded_rect, &border_color, button_border_width);
        cx.fill(rounded_rect, &bg_gradient);
        let layout = self.layout.as_ref().unwrap();
        let offset = (cx.size().to_vec2() - layout.size().to_vec2()) * 0.5;
        cx.draw_text(layout, offset.to_point());
    }
}
