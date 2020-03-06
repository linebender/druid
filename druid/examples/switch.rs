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

use druid::widget::{Flex, Label, Padding, Parse, Stepper, Switch, TextBox, WidgetExt};
use druid::{AppLauncher, Data, Lens, LensExt, LensWrap, LocalizedString, Widget, WindowDesc};

#[derive(Clone, Data, Lens)]
struct DemoState {
    value: bool,
    stepper_value: f64,
}

fn build_widget() -> impl Widget<DemoState> {
    let mut col = Flex::column();
    let mut row = Flex::row();
    let switch = LensWrap::new(Switch::new(), DemoState::value);
    let switch_label = Label::new("Setting label");

    row.add_child(Padding::new(5.0, switch_label), 0.0);
    row.add_child(Padding::new(5.0, switch), 0.0);

    let stepper = LensWrap::new(
        Stepper::new().max(10.0).min(0.0).step(0.5).wrap(false),
        DemoState::stepper_value,
    );

    let mut textbox_row = Flex::row();
    let textbox = LensWrap::new(
        Parse::new(TextBox::new()),
        DemoState::stepper_value.map(|x| Some(*x), |x, y| *x = y.unwrap_or(0.0)),
    );
    textbox_row.add_child(Padding::new(5.0, textbox), 0.0);
    textbox_row.add_child(Padding::new(5.0, stepper.center()), 0.0);

    let mut label_row = Flex::row();

    let label = Label::new(|data: &DemoState, _env: &_| {
        format!("Stepper value: {0:.2}", data.stepper_value)
    });

    label_row.add_child(Padding::new(5.0, label), 0.0);

    col.add_child(Padding::new(5.0, row), 1.0);
    col.add_child(Padding::new(5.0, textbox_row), 0.0);
    col.add_child(Padding::new(5.0, label_row), 1.0);
    col.debug_paint_layout()
}

fn main() {
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
