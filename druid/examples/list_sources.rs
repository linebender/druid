// Copyright 2021 The Druid Authors.
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

//! Demos alternative iterable data type for the List widget, im::OrdMap
//! Two ListIter implementations provide a version which only concerns
//! itself with the values, and another which concerns itself with both
//! keys and vales.

use druid::im::{ordmap, OrdMap};
use druid::widget::{Button, CrossAxisAlignment, Flex, Label, List, Scroll};
use druid::{
    AppLauncher, Color, Data, Lens, LocalizedString, UnitPoint, Widget, WidgetExt, WindowDesc,
};

#[derive(Clone, Data, Lens)]
struct AppData {
    adding_index: usize,
    om_values: OrdMap<u32, String>,
    om_keys_values: OrdMap<String, u64>,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder())
        .window_size((1200.0, 600.0))
        .title(
            LocalizedString::new("list-sources-demo-window-title")
                .with_placeholder("List Sources Demo"),
        );
    // Set our initial data.
    let om_values = ordmap! {3 => String::from("Apple"), 1 => String::from("Pear"), 2 => String::from("Orange")};
    let om_keys_values = ordmap! {String::from("Russia") => 17098242, String::from("Canada") => 9984670,
    String::from("China") => 956960};
    let data = AppData {
        adding_index: om_values.len(),
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

    //Buttons to test adding and removing entries from the lists
    root.add_child(
        Button::new("Add")
            .on_click(|_, data: &mut AppData, _| {
                data.adding_index += 1;
                data.om_values.insert(
                    data.adding_index as u32,
                    format!("Fruit #{}", data.adding_index),
                );
                data.om_keys_values
                    .insert(format!("Country #{}", data.adding_index), 42);
            })
            .fix_height(30.0)
            .expand_width(),
    );
    root.add_child(
        Button::new("Remove First")
            .on_click(|_, data: &mut AppData, _| {
                if !data.om_values.is_empty() {
                    if let Some(k) = data.om_values.clone().iter().next() {
                        data.om_values.remove(&k.0.clone());
                    }
                }
                if !data.om_keys_values.is_empty() {
                    if let Some(k) = data.om_keys_values.clone().iter().next() {
                        data.om_keys_values.remove(&k.0.clone());
                    }
                }
            })
            .fix_height(30.0)
            .expand_width(),
    );

    let mut lists = Flex::row().cross_axis_alignment(CrossAxisAlignment::Start);

    // Build a list values from an ordmap
    // The display order will be based on the Ord trait of the keys
    lists.add_flex_child(
        Flex::column()
            .with_child(Label::new("List from im::OrdMap Values"))
            .with_child(
                Scroll::new(List::new(|| {
                    Label::new(|item: &String, _env: &_| item.to_string())
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

    root.add_flex_child(lists, 1.0);

    root
}
