// Copyright 2019 The Druid Authors.
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

//! Demos virtualized list widget.

use druid::im::{vector, Vector};
use druid::widget::{Button, Flex, Label, VirtList};
use druid::{AppLauncher, Data, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};

#[derive(Clone, Data, Lens)]
struct AppData {
    list: Vector<String>,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("list-demo-window-title").with_placeholder("VirtList Demo"));
    // Set our initial data
    let data = AppData { list: vector![] };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();

    // Build buttons to add/remove children from the list
    root.add_child(
        Button::new("Add")
            .on_click(|_, data: &mut AppData, _| {
                // Add 10 items to the list.
                for _ in 0..10 {
                    data.list.push_back(data.list.len().to_string());
                }
            })
            .fix_height(30.0)
            .expand_width(),
    );
    root.add_child(
        Button::new("Remove")
            .on_click(|_, data: &mut AppData, _| {
                // Pop 10 items from the list.
                for _ in 0..10 {
                    data.list.pop_back();
                }
            })
            .fix_height(30.0)
            .expand_width(),
    );

    // Add the virtualized list container.
    const CHILD_HEIGHT: f64 = 30.0;
    root.add_flex_child(
        VirtList::vertical(CHILD_HEIGHT, || Label::raw().fix_height(CHILD_HEIGHT))
            .lens(AppData::list),
        1.0,
    );

    // Mark the widget as needing its layout rects painted
    root.debug_paint_layout()
}
