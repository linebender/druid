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

//! Shows a scroll widget, and also demonstrates how widgets that paint
//! outside their bounds can specify their paint region.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::lens::Unit;
use druid::widget::prelude::*;
use druid::widget::{BackgroundBrush, Button, ClipBox, Flex, Label, List, Padding, Side, Slider, Tabs, TextBox, ViewportHeader};
use druid::{
    AppLauncher, Color, Data, Insets, Lens, LocalizedString, Point, Rect, RoundedRectRadii, Vec2,
    WidgetExt, WidgetPod, WindowDesc,
};
use im::Vector;
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
struct AppData {
    list: Vector<Contact>,
}

#[derive(Clone, Data, Lens)]
struct Contact {
    name: Arc<String>,
    info: Vector<Arc<String>>,
}

pub fn main() {
    let window = WindowDesc::new(build_widget())
        .title(LocalizedString::new("scroll-demo-window-title").with_placeholder("Scroll demo"));

    let mut list = Vector::new();
    list.push_back(Arc::new("test".to_string()));
    list.push_back(Arc::new("test2".to_string()));
    list.push_back(Arc::new("test3".to_string()));

    AppLauncher::with_window(window)
        .log_to_console()
        .launch(AppData {
            list: Vector::new(),
        })
        .expect("launch failed");
}

fn build_widget() -> impl Widget<AppData> {
    let list = List::new(|| {
        let body = Flex::column()
            .with_default_spacer()
            .with_child(Label::new("Name:").align_left())
            .with_default_spacer()
            .with_child(TextBox::new().lens(Contact::name).expand_width())
            .with_default_spacer()
            .with_default_spacer()
            .with_child(Label::new("Info:").align_left())
            .with_default_spacer()
            .with_child(List::new(|| TextBox::new().padding(Insets::new(15.0, 0.0, 0.0, 10.0)).expand_width()).lens(Contact::info))
            .with_child(
                Button::new("Add Info").on_click(|_, data: &mut Contact, _| {
                    data.info.push_back(Arc::new(String::new()))
                }),
            )
            .with_default_spacer()
            .align_left()
            .padding(Insets::uniform_xy(25.0, 0.0))
            .background(Color::grey8(25))
            .rounded(RoundedRectRadii::new(0.0, 0.0, 5.0, 5.0));

        ViewportHeader::new(
            body,
            Label::dynamic(|data: &Contact, _| format!("Contact \"{}\"", &data.name))
                .center()
                .background(Color::grey8(15))
                .rounded(RoundedRectRadii::new(5.0, 5.0, 0.0, 0.0)),
            Side::Top,
        )
        .clipped_content(true)
        .with_minimum_visible_content(20.0)
        .padding(Insets::uniform_xy(0.0, 5.0))
    })
    .lens(AppData::list)
    .scroll()
    .vertical();

    Flex::column()
        .with_flex_child(list, 1.0)
        .with_default_spacer()
        .with_child(
            Button::new("Add Contact").on_click(|_, data: &mut AppData, _| {
                data.list.push_back(Contact {
                    name: Arc::new("New Contact".to_string()),
                    info: Default::default(),
                })
            }),
        )
}
