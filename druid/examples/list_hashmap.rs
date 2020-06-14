// Copyright 2020 The xi-editor Authors.
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

//! Demos basic list widget and list manipulations using im's HashMap.

#[cfg(not(feature = "im"))]
pub fn main() {
    eprintln!("This examples requires the \"im\" feature to be enabled:");
    eprintln!("cargo run --example list_hashmap --features im");
}

#[cfg(feature = "im")]
pub fn main() {
    example::main()
}

#[cfg(feature = "im")]
mod example {
    use druid::lens::{self, LensExt};
    use druid::widget::{Button, CrossAxisAlignment, Flex, Label, List, Scroll};
    use druid::{
        AppLauncher, Color, Data, Lens, LocalizedString, UnitPoint, Widget, WidgetExt, WindowDesc,
    };

    #[derive(Clone, Data, Lens)]
    struct AppData {
        left_map: im::HashMap<String, u32>,
        right_map: im::HashMap<String, u32>,
    }

    pub fn main() {
        let main_window = WindowDesc::new(ui_builder)
            .title(LocalizedString::new("list-demo-window-title").with_placeholder("List Demo"));
        // Set our initial data
        let mut map = im::HashMap::new();
        map.insert("One".to_string(), 1);
        map.insert("Two".to_string(), 2);
        map.insert("Three".to_string(), 3);
        let data = AppData {
            left_map: map.clone(),
            right_map: map,
        };
        AppLauncher::with_window(main_window)
            .use_simple_logger()
            .launch(data)
            .expect("launch failed");
    }

    fn ui_builder() -> impl Widget<AppData> {
        let mut root = Flex::column();

        // Build a button to add children to both lists
        root.add_child(
            Button::new("Add")
                .on_click(|_, data: &mut AppData, _| {
                    // Inserting into our HashMaps by finding the highest key and going + 1
                    // Add child to left list
                    let left_max = data
                        .left_map
                        .iter()
                        .max_by_key(|(_, value)| value.clone())
                        .unwrap();
                    let value = left_max.1 + 1;
                    data.left_map
                        .insert(format!("{}", value).to_string(), value as u32);

                    // Add child to left list
                    let right_max = data
                        .right_map
                        .iter()
                        .max_by_key(|(_, value)| value.clone())
                        .unwrap();
                    let value = right_max.1 + 1;
                    data.right_map
                        .insert(format!("{}", value).to_string(), value as u32);
                })
                .fix_height(30.0)
                .expand_width(),
        );

        let mut lists = Flex::row().cross_axis_alignment(CrossAxisAlignment::Start);

        // Build a simple list
        lists.add_flex_child(
            Scroll::new(List::new(|| {
                Label::new(|(item_key, item_value): &(String, u32), _env: &_| {
                    format!("List item key:{}, value:{}", item_key, item_value)
                })
                .align_vertical(UnitPoint::LEFT)
                .padding(10.0)
                .expand()
                .height(50.0)
                .background(Color::rgb(0.5, 0.5, 0.5))
            }))
            .vertical()
            .lens(AppData::left_map),
            1.0,
        );

        // Build a list with shared data
        lists.add_flex_child(
            Scroll::new(List::new(|| {
                Flex::row()
                    .with_child(
                        Label::new(
                            |(_, item_key, item_value): &(
                                im::HashMap<String, u32>,
                                String,
                                u32,
                            ),
                             _env: &_| {
                                format!("List item key:{}, value:{}", item_key, item_value)
                            },
                        )
                        .align_vertical(UnitPoint::LEFT),
                    )
                    .with_flex_spacer(1.0)
                    .with_child(
                        Button::new("Delete")
                            .on_click(
                                |_ctx,
                                 (shared, item_key, _item_value): &mut (
                                    im::HashMap<String, u32>,
                                    String,
                                    u32,
                                ),
                                 _env| {
                                    // We have access to both child's data and shared data.
                                    // Remove element from right list.
                                    shared.remove(item_key).expect("That item wasn't found");
                                },
                            )
                            .fix_size(80.0, 20.0)
                            .align_vertical(UnitPoint::CENTER),
                    )
                    .padding(10.0)
                    .background(Color::rgb(0.5, 0.0, 0.5))
                    .fix_height(50.0)
            }))
            .vertical()
            .lens(lens::Id.map(
                // Expose shared data with children data
                |d: &AppData| (d.right_map.clone(), d.right_map.clone()),
                |d: &mut AppData, x: (im::HashMap<String, u32>, im::HashMap<String, u32>)| {
                    // If shared data was changed reflect the changes in our AppData
                    d.right_map = x.0
                },
            )),
            1.0,
        );

        root.add_flex_child(lists, 1.0);

        // Mark the widget as needing its layout rects painted
        root.debug_paint_layout()
    }
}
