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

//! Demos basic list widget and list manipulations.

use std::sync::Arc;

use druid::lens::{self, LensExt};
use druid::widget::{Button, Flex, Label, List, Scroll, WidgetExt};
use druid::{AppLauncher, Color, Data, Lens, LocalizedString, UnitPoint, Widget, WindowDesc};

#[derive(Clone, Data, Lens)]
struct AppData {
    left: Arc<Vec<u32>>,
    right: Arc<Vec<u32>>,
}

fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("list-demo-window-title").with_placeholder("List Demo"));
    // Set our initial data
    let data = AppData {
        left: Arc::new(vec![1, 2]),
        right: Arc::new(vec![1, 2, 3]),
    };
    AppLauncher::with_window(main_window)
        .debug_paint_layout()
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();

    // Build a button to add children to both lists
    root.add_child(
        Button::new("Add", |_, data: &mut AppData, _| {
            // Add child to left list
            let value = data.left.len() + 1;
            Arc::make_mut(&mut data.left).push(value as u32);

            // Add child to right list
            let value = data.right.len() + 1;
            Arc::make_mut(&mut data.right).push(value as u32);
        })
        .fix_height(30.0),
        0.0,
    );

    let mut lists = Flex::row();

    // Build a simple list
    lists.add_child(
        Scroll::new(List::new(|| {
            Label::new(|item: &u32, _env: &_| format!("List item #{}", item))
                .padding(10.0)
                .expand()
                .height(50.0)
                .background(Color::rgb(0.5, 0.5, 0.5))
        }))
        .vertical()
        .lens(AppData::left),
        1.0,
    );

    // Build a list with shared data
    lists.add_child(
        Scroll::new(List::new(|| {
            Flex::row()
                .with_child(
                    Label::new(|(_, item): &(Arc<Vec<u32>>, u32), _env: &_| {
                        format!("List item #{}", item)
                    }),
                    1.0,
                )
                .with_child(
                    Button::new(
                        "Delete",
                        |_ctx, (shared, item): &mut (Arc<Vec<u32>>, u32), _env| {
                            // We have access to both child's data and shared data.
                            // Remove element from right list.
                            Arc::make_mut(shared).retain(|v| v != item);
                        },
                    )
                    .fix_size(80.0, 20.0)
                    .align_vertical(UnitPoint::CENTER),
                    0.0,
                )
                .padding(10.0)
                .background(Color::rgb(0.5, 0.0, 0.5))
                .fix_height(50.0)
        }))
        .vertical()
        .lens(lens::Id.map(
            // Expose shared data with children data
            |d: &AppData| (d.right.clone(), d.right.clone()),
            |d: &mut AppData, x: (Arc<Vec<u32>>, Arc<Vec<u32>>)| {
                // If shared data was changed reflect the changes in our AppData
                d.right = x.0
            },
        )),
        1.0,
    );

    root.add_child(lists, 1.0);

    root
}
