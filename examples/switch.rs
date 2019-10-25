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

use druid::widget::{
    Align, Button, Checkbox, Column, DynLabel, Label, Padding, ProgressBar, Row, Switch,
};
use druid::{AppLauncher, Data, Lens, LensWrap, Widget, WindowDesc};

#[derive(Clone, Data, Lens)]
struct DemoState {
    value: bool
}

fn build_widget() -> impl Widget<DemoState> {
    let mut col = Column::new();
    let label = DynLabel::new(|data: &DemoState, _env| {
        format!("value: {}", data.value)
    });
    let mut row = Row::new();
    let switch = LensWrap::new(Switch::new(), lenses::demo_state::value);
    let switch_label = Label::new("Enable");

    row.add_child(Padding::uniform(5.0, switch_label), 0.0);
    row.add_child(Padding::uniform(5.0, switch), 1.0);

    col.add_child(Padding::uniform(5.0, row), 1.0);
    col
}

fn main() {
    let window = WindowDesc::new(build_widget);
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(DemoState {
            value: true
        })
        .expect("launch failed");
}
