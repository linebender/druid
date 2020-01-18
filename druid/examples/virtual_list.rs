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

use std::cell::RefCell;
use std::sync::Arc;

use druid::widget::{
    Flex, ListData, Padding, ScrollControlState, Scrollbar, VirtualList, WidgetExt,
};
use druid::{AppLauncher, Data, Insets, Lens, Widget, WindowDesc};

#[derive(Clone, Data, Lens, PartialEq, Default)]
struct VirtualScrollState {
    id: u64,
    last_mouse_pos: f64,
    page_size: f64,
    max_scroll_position: f64,
    min_scroll_position: f64,
    mouse_wheel_enabled: bool,
    scroll_position: f64,
    tracking_mouse: bool,
    scale: f64,
}

impl ScrollControlState for VirtualScrollState {
    fn last_mouse_pos(&self) -> f64 {
        self.last_mouse_pos
    }

    fn id(&self) -> u64 {
        self.id
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

    fn set_scale(&mut self, val: f64) {
        self.scale = val;
    }

    fn set_scroll_position(&mut self, val: f64) {
        self.scroll_position = val;
    }
}

#[derive(Clone, Data, Lens, PartialEq, Default)]
struct VirtualListData {
    state: RefCell<VirtualScrollState>,
    data: Arc<Vec<String>>,
}

impl ListData<String, VirtualScrollState> for VirtualListData {
    fn get_scroll_control_state(&self) -> &RefCell<VirtualScrollState> {
        &self.state
    }

    fn get_data(&self) -> Arc<Vec<String>> {
        self.data.clone()
    }
}

fn main() {
    let window = WindowDesc::new(build_widget);
    let mut items = Vec::new();
    for i in 0..1_000_000 {
        items.push(format!("List Item {}", i));
    }
    let list_data = VirtualListData {
        state: RefCell::new(VirtualScrollState {
            id: 1,
            last_mouse_pos: 0.,
            page_size: 0.,
            scroll_position: 0.0,
            max_scroll_position: 0.,
            min_scroll_position: 0.,
            mouse_wheel_enabled: true,
            tracking_mouse: false,
            scale: 0.,
        }),
        data: Arc::new(items),
    };

    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(list_data)
        .expect("launch failed");

    fn build_widget() -> impl Widget<VirtualListData> {
        let v_list = VirtualList::new();
        let scrollbar = Scrollbar::new().fix_width(8.).lens(VirtualListData::state);
        Flex::row()
            .with_child(v_list, 1.)
            .with_child(Padding::new(Insets::new(0., 2., 5., 4.), scrollbar), 0.)
    }
}
