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

use druid::widget::{Flex, Switch, DynLabel, Label, Padding, Row, Stepper, Switch};
use druid::{AppLauncher, Data, Lens, LensWrap, Widget, WindowDesc};

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


    let label_stepper = LensWrap::new(
        Stepper::new(0.0, 10.0, 1.0, true, |_ctx, _data, _env| {}),
        lenses::demo_state::stepper_value,
    );

    let mut stepper_row = Row::new();

    let label = DynLabel::new(|data: &DemoState, _env| {
        format!("Stepper value: {0:.0}", data.stepper_value)
    });

    stepper_row.add_child(Padding::new(5.0, label), 0.0);
    stepper_row.add_child(Padding::new(5.0, label_stepper), 0.0);

    col.add_child(Padding::new(5.0, row), 1.0);
    col.add_child(Padding::new(5.0, stepper_row), 1.0);
    col
}

fn main() {
    let window = WindowDesc::new(build_widget);
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(DemoState {
            value: true,
            stepper_value: 1.0,
        })
        .expect("launch failed");
}
