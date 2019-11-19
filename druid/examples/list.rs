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

use std::ops::Deref;
use std::sync::Arc;

use druid::piet::Color;

use druid::widget::{Button, Column, Container, DynLabel, List, Padding, Scroll, SizedBox};
use druid::{AppLauncher, Widget, WindowDesc};

type AppData = Arc<Vec<u32>>;

fn main() {
    let main_window = WindowDesc::new(ui_builder);
    let data = Arc::new(vec![1, 2, 3, 4]);
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    let mut root = Column::new();

    root.add_child(
        SizedBox::new(Button::new("Add", |_, data: &mut AppData, _| {
            let mut d = (*data).deref().to_owned();
            d.push((d.len() + 1) as u32);
            *data = d.into();
        }))
        .height(30.0),
        0.0,
    );

    root.add_child(
        Scroll::new(List::new(|| {
            Box::new(
                SizedBox::new(
                    Container::new(Padding::new(
                        10.0,
                        DynLabel::new(|d, _| format!("List item #{}", d)),
                    ))
                    .background(Color::rgb(0.5, 0.5, 0.5)),
                )
                .expand()
                .height(50.0),
            )
        }))
        .vertical(),
        1.0,
    );

    root
}
