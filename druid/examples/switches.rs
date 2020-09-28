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

use druid::widget::{
    Checkbox, Flex, Label, LensWrap, MainAxisAlignment, Padding, Parse, Stepper, Switch, TextBox,
    WidgetExt,
};
use druid::{AppLauncher, Data, Lens, LensExt, LocalizedString, Widget, WindowDesc};

#[derive(Clone, Data, Lens)]
struct DemoState {
    value: bool,
    stepper_value: f64,
}

fn build_widget() -> impl Widget<DemoState> {
    let mut col = Flex::column();
    let mut row = Flex::row();
    let switch = LensWrap::new(Switch::new(), DemoState::value);
    let check_box = LensWrap::new(Checkbox::new(""), DemoState::value);
    let switch_label = Label::new("Setting label");

    row.add_child(Padding::new(5.0, switch_label));
    row.add_child(Padding::new(5.0, switch));
    row.add_child(Padding::new(5.0, check_box));

    let stepper = LensWrap::new(
        Stepper::new()
            .with_range(0.0, 10.0)
            .with_step(0.5)
            .with_wraparound(false),
        DemoState::stepper_value,
    );

    let mut textbox_row = Flex::row();
    let textbox = LensWrap::new(
        Parse::new(TextBox::new()),
        DemoState::stepper_value.map(|x| Some(*x), |x, y| *x = y.unwrap_or(0.0)),
    );
    textbox_row.add_child(Padding::new(5.0, textbox));
    textbox_row.add_child(Padding::new(5.0, stepper.center()));

    let mut label_row = Flex::row();

    let label = Label::new(|data: &DemoState, _env: &_| {
        format!("Stepper value: {0:.2}", data.stepper_value)
    });

    label_row.add_child(Padding::new(5.0, label));

    col.set_main_axis_alignment(MainAxisAlignment::Center);
    col.add_child(Padding::new(5.0, row));
    col.add_child(Padding::new(5.0, textbox_row));
    col.add_child(Padding::new(5.0, label_row));
    col.center()
}

pub fn main() {
    let window = WindowDesc::new(build_widget)
        .title(LocalizedString::new("switch-demo-window-title").with_placeholder("Switch Demo"));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(DemoState {
            value: true,
            stepper_value: 1.0,
        })
        .expect("launch failed");
}
