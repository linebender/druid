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
    piet::{Color, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder},
};

use super::{
    align::{FirstBaseline, LastBaseline, SingleAlignment, VertAlignment},
    AlignCx, EventCx, LayoutCx, PaintCx, UpdateCx, Widget,
};

pub struct TextWidget {
    text: String,
    color: Color,
    layout: Option<PietTextLayout>,
    is_wrapped: bool,
}

impl TextWidget {
    pub fn new(text: String) -> TextWidget {
        TextWidget {
            text,
            color: Color::WHITE,
            layout: None,
            is_wrapped: false,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.layout = None;
    }
}

impl Widget for TextWidget {
    fn event(&mut self, _cx: &mut EventCx, _event: &super::RawEvent) {}

    fn update(&mut self, cx: &mut UpdateCx) {
        // All changes potentially require layout. Note: we could be finer
        // grained, maybe color changes wouldn't.
        cx.request_layout();
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        let layout = cx
            .text()
            .new_text_layout(self.text.clone())
            .text_color(self.color.clone())
            .build()
            .unwrap();
        let min_size = Size::ZERO;
        let max_size = layout.size();
        self.layout = Some(layout);
        self.is_wrapped = false;
        (min_size, max_size)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        let needs_wrap = proposed_size.width < cx.widget_state.max_size.width;
        if self.is_wrapped || needs_wrap {
            let layout = cx
                .text()
                .new_text_layout(self.text.clone())
                .max_width(proposed_size.width)
                .text_color(self.color.clone())
                .build()
                .unwrap();
            let size = layout.size();
            self.layout = Some(layout);
            self.is_wrapped = needs_wrap;
            size
        } else {
            cx.widget_state.max_size
        }
    }

    fn align(&self, cx: &mut AlignCx, alignment: SingleAlignment) {
        if alignment.id() == FirstBaseline.id() {
            if let Some(metric) = self.layout.as_ref().unwrap().line_metric(0) {
                cx.aggregate(alignment, metric.baseline);
            }
        } else if alignment.id() == LastBaseline.id() {
            let i = self.layout.as_ref().unwrap().line_count() - 1;
            if let Some(metric) = self.layout.as_ref().unwrap().line_metric(i) {
                cx.aggregate(alignment, metric.y_offset + metric.baseline);
            }
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        cx.draw_text(self.layout.as_ref().unwrap(), Point::ZERO);
    }
}
