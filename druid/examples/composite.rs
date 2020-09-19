// Copyright 2020 The Druid Authors.
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

//! Demonstrates alignment of children in the flex container.

use druid::widget::prelude::*;
use druid::widget::{
    Align, Button, Checkbox, CompositeBuild, CompositeWidget, CrossAxisAlignment, Flex, Label,
    MainAxisAlignment, ProgressBar, RadioGroup, SizedBox, Slider, Stepper, Switch, TextBox,
    WidgetExt,
};
use druid::{AppLauncher, LocalizedString, WindowDesc};

pub struct TextBoxWithLabel {
    label: String,
}

impl TextBoxWithLabel {
    fn new(label: impl Into<String>) -> Self {
        TextBoxWithLabel {
            label: label.into(),
        }
    }
}

impl CompositeBuild<String, Flex<String>> for TextBoxWithLabel {
    fn build(&self) -> Flex<String> {
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(Label::new(self.label.clone()))
            .with_spacer(4.)
            .with_child(TextBox::new().with_placeholder(String::from("Test")))
    }
}

fn make_ui() -> impl Widget<String> {
    Align::centered(
        Flex::column()
            .with_child(CompositeWidget::new(TextBoxWithLabel::new("My text box")))
            .padding(10.0),
    )
}

pub fn main() {
    let main_window = WindowDesc::new(make_ui)
        .window_size((720., 600.00))
        .with_min_size((620., 265.00))
        .title(LocalizedString::new("Composite widget example"));

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(String::new())
        .unwrap();
}
