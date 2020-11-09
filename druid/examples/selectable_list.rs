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

use druid::{
    lens,
    lens::Id,
    widget::{Either, Label, List},
    AppLauncher, Color, Data, Lens, LensExt, Widget, WidgetExt, WindowDesc,
};
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
struct AppData {
    // We have to store our index
    list_labels: Arc<Vec<ListItem>>,
    selected: Option<usize>,
}

#[derive(Clone, Data, Lens)]
struct ListItem {
    label: String,
    // we need to keep a copy of the index inside the struct, to have access to it within the list.
    position: usize,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title("Selectable list");
    // Set our initial data
    let data = AppData {
        list_labels: Arc::new(
            vec!["label 1".into(), "label 2".into(), "label 3".into()]
                .into_iter()
                .enumerate()
                .map(|(position, label)| ListItem { label, position })
                .collect(),
        ),
        selected: Some(0),
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    let l = Id.map(
        |data: &AppData| (data.selected, data.list_labels.clone()),
        |data, (selected, list_labels)| {
            data.selected = selected;
            data.list_labels = list_labels;
        },
    );
    List::vertical(list_item).center().lens(l)
}

fn list_item() -> impl Widget<(Option<usize>, ListItem)> {
    let l = lens!((Option<usize>, ListItem), 1).then(lens!(ListItem, label));
    let l2 = lens!((Option<usize>, ListItem), 1).then(lens!(ListItem, label));
    Either::new(
        |data, _| data.0 == Some(data.1.position),
        Label::raw().with_text_color(Color::BLACK).lens(l),
        Label::raw().lens(l2),
    )
    .on_click(|_, data, _| {
        if data.0 == Some(data.1.position) {
            data.0 = None;
        } else {
            data.0 = Some(data.1.position);
        }
    })
}
