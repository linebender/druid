// Copyright 2019 The xi-editor Authors.
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

use druid::{AppLauncher, Data, Lens, Widget, WindowDesc};
use druid::widget::{VirtualList, Flex, Scrollbar, WidgetExt, ScrollControlState};

#[derive(Clone, Data, Lens, PartialEq)]
struct VirtualScrollState {
    last_mouse_pos: f64,
    page_size: f64,
    max_scroll_position: f64,
    min_scroll_position: f64,
    mouse_wheel_enabled: bool,
    scroll_position: f64,
    tracking_mouse: bool,
    scale: f64
}

impl ScrollControlState for VirtualScrollState {
    fn last_mouse_pos(&self) -> f64 {
        self.last_mouse_pos
    }

    fn page_size(&self) -> f64 {
        self.page_size
    }

    fn max_scroll_position(&self) -> f64 {
        self.max_scroll_position
    }

    fn min_scroll_position(&self) -> f64 {
        self.min_scroll_position
    }

    fn mouse_wheel_enabled(&self) -> bool {
        self.mouse_wheel_enabled
    }

    fn scale(&self) -> f64 {
        self.scale
    }

    fn scroll_position(&self) -> f64 {
        self.scroll_position
    }

    fn tracking_mouse(&self) -> bool {
        self.tracking_mouse
    }

    fn set_last_mouse_pos(&mut self, val: f64) {
        self.last_mouse_pos = val;
    }

    fn set_page_size(&mut self, val: f64) {
        self.page_size = val;
    }

    fn set_max_scroll_position(&mut self, val: f64) {
        self.max_scroll_position = val;
    }

    fn set_min_scroll_position(&mut self, val: f64) {
        self.min_scroll_position = val;
    }

    fn set_mouse_wheel_enabled(&mut self, val: bool) {
        self.mouse_wheel_enabled = val;
    }

    fn set_tracking_mouse(&mut self, val: bool) {
       self.tracking_mouse = val;
    }

    fn set_scale(&mut self, val:f64) {
        self.scale = val;
    }

    fn set_scroll_position(&mut self, val: f64) {
        self.scroll_position = val;
    }
}

fn main() {
    let window = WindowDesc::new(build_widget);
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(VirtualScrollState {
            last_mouse_pos: 0.,
            page_size: 1.0,
            scroll_position: 0.0,
            max_scroll_position: (1000000. * 30.) - 400.,
            min_scroll_position: 0.0,
            mouse_wheel_enabled: true,
            tracking_mouse: false,
            scale: 0.
        }).expect("launch failed");

    fn build_widget() -> impl Widget<VirtualScrollState> {
        let mut data = Vec::new();
        for i in 0..1000000 {
            data.push(format!("List Item {}", i));
        }
        Flex::row()
            .with_child(VirtualList::new().data_provider(data), 1.)
            .with_child(Scrollbar::new().fix_width(20.), 1.)
    }
}
