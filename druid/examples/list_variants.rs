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

//! Demos alternative iterable data types for the List widget.

use druid::im::{hashmap, ordmap, vector, HashMap, OrdMap, Vector};
use druid::lens::{self, LensExt};
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, List, Scroll};
use druid::{
    AppLauncher, Color, Data, Lens, LocalizedString, UnitPoint, Widget, WidgetExt, WindowDesc,
};

#[derive(Clone, Data, Lens)]
struct AppData {
    hm_values: HashMap<u32, String>,
    hm_keys_values: HashMap<String, u64>,
    om_values: OrdMap<u32, String>,
    om_keys_values: OrdMap<String, u64>,
    hm_values_index: usize,
    hm_keys_values_index: usize,
    om_values_index: usize,
    om_keys_values_index: usize,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder())
        .window_size((1200.0, 600.0))
        .title(
            LocalizedString::new("list-variants-demo-window-title")
                .with_placeholder("List Variants Demo"),
        );
    // Set our initial data
    let hm_values = hashmap! {3 => String::from("Apple"), 1 => String::from("Pear"), 2 => String::from("Orange")};
    let hm_keys_values = hashmap! {String::from("Russia") => 17098242, String::from("Canada") => 9984670,
    String::from("China") => 956960};
    let om_values = ordmap! {3 => String::from("Apple"), 1 => String::from("Pear"), 2 => String::from("Orange")};
    let om_keys_values = ordmap! {String::from("Russia") => 17098242, String::from("Canada") => 9984670,
    String::from("China") => 956960};
    let data = AppData {
        hm_values_index: hm_values.len(),
        hm_keys_values_index: hm_keys_values.len(),
        om_values_index: om_values.len(),
        om_keys_values_index: om_keys_values.len(),
        hm_values,
        hm_keys_values,
        om_values,
        om_keys_values,
    };
    AppLauncher::with_window(main_window)
        .use_env_tracing()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();

    // Build a button to add children to both lists
    // root.add_child(
    //     Button::new("Add")
    //         .on_click(|_, data: &mut AppData, _| {
    //             // Add child to left list
    //             data.l_index += 1;
    //             data.hm_values
    //                 .insert(data.l_index as u32, String::from("Another Fruit"));

    //             // Add child to right list
    //             data.r_index += 1;
    //             data.right.push_back(data.r_index as u32);
    //         })
    //         .fix_height(30.0)
    //         .expand_width(),
    // );

    let mut lists = Flex::row().cross_axis_alignment(CrossAxisAlignment::Start);

    // Build a list of values from a hashmap
    // The display order will be indeterminate.
    lists.add_flex_child(
        Flex::column()
            .with_child(Label::new("List from im::HashMap Values"))
            .with_child(
                Scroll::new(List::new(|| {
                    Label::new(|item: &String, _env: &_| format!("{}", item))
                        .align_vertical(UnitPoint::LEFT)
                        .padding(10.0)
                        .expand()
                        .height(50.0)
                        .background(Color::rgb(0.5, 0.5, 0.5))
                }))
                .vertical()
                .lens(AppData::hm_values),
            ),
        1.0,
    );

    // Build a list of key value pairs from a hashmap
    // The display order will be indeterminate.
    lists.add_flex_child(
        Flex::column()
            .with_child(Label::new("List from im::HashMap Keys and Values"))
            .with_child(
                Scroll::new(List::new(|| {
                    Label::new(|item: &(String, u64), _env: &_| {
                        format!("{0}: {1} square kilometres", item.0, item.1)
                    })
                    .align_vertical(UnitPoint::LEFT)
                    .padding(10.0)
                    .expand()
                    .height(50.0)
                    .background(Color::rgb(0.5, 0.0, 0.5))
                }))
                .vertical()
                .lens(AppData::hm_keys_values),
            ),
        1.0,
    );

    // Build a list values from an ordmap
    // The display order will be based on the Ord trait of the keys
    lists.add_flex_child(
        Flex::column()
            .with_child(Label::new("List from im::OrdMap Values"))
            .with_child(
                Scroll::new(List::new(|| {
                    Label::new(|item: &String, _env: &_| format!("{}", item))
                        .align_vertical(UnitPoint::LEFT)
                        .padding(10.0)
                        .expand()
                        .height(50.0)
                        .background(Color::rgb(0.5, 0.5, 0.5))
                }))
                .vertical()
                .lens(AppData::om_values),
            ),
        1.0,
    );

    // Build a list of key value pairs from an ordmap
    // The display order will be based on the Ord trait of the keys
    lists.add_flex_child(
        Flex::column()
            .with_child(Label::new("List from im::OrdMap Keys and Values"))
            .with_child(
                Scroll::new(List::new(|| {
                    Label::new(|item: &(String, u64), _env: &_| {
                        format!("{0}: {1} square kilometres", item.0, item.1)
                    })
                    .align_vertical(UnitPoint::LEFT)
                    .padding(10.0)
                    .expand()
                    .height(50.0)
                    .background(Color::rgb(0.5, 0.0, 0.5))
                }))
                .vertical()
                .lens(AppData::om_keys_values),
            ),
        1.0,
    );

    // // Build a list with shared data
    // lists.add_flex_child(
    //     Scroll::new(
    //         List::new(|| {
    //             Flex::row()
    //                 .with_child(
    //                     Label::new(|(_, item): &(Vector<u32>, u32), _env: &_| {
    //                         format!("List item #{}", item)
    //                     })
    //                     .align_vertical(UnitPoint::LEFT),
    //                 )
    //                 .with_flex_spacer(1.0)
    //                 .with_child(
    //                     Button::new("Delete")
    //                         .on_click(|_ctx, (shared, item): &mut (Vector<u32>, u32), _env| {
    //                             // We have access to both child's data and shared data.
    //                             // Remove element from right list.
    //                             shared.retain(|v| v != item);
    //                         })
    //                         .fix_size(80.0, 20.0)
    //                         .align_vertical(UnitPoint::CENTER),
    //                 )
    //                 .padding(10.0)
    //                 .background(Color::rgb(0.5, 0.0, 0.5))
    //                 .fix_height(50.0)
    //         })
    //         .with_spacing(10.),
    //     )
    //     .vertical()
    //     .lens(lens::Identity.map(
    //         // Expose shared data with children data
    //         |d: &AppData| (d.right.clone(), d.right.clone()),
    //         |d: &mut AppData, x: (Vector<u32>, Vector<u32>)| {
    //             // If shared data was changed reflect the changes in our AppData
    //             d.right = x.0
    //         },
    //     )),
    //     1.0,
    // );

    root.add_flex_child(lists, 1.0);

    root
}
