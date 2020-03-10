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
    Align, Alignment, Button, Checkbox, Flex, Label, Padding, ProgressBar, Slider, WidgetExt,
};
use druid::{AppLauncher, Data, Lens, LensWrap, LocalizedString, UnitPoint, Widget, WindowDesc};

#[derive(Clone, Data, Lens)]
struct DemoState {
    value: f64,
    double: bool,
}

fn build_widget() -> impl Widget<DemoState> {
    let label = Label::new(|data: &DemoState, _env: &_| {
        if data.double {
            format!("2x the value: {0:.2}", data.value * 2.0)
        } else {
            format!("actual value: {0:.2}", data.value)
        }
    });
    let checkbox = LensWrap::new(Checkbox::new(), DemoState::double);
    let checkbox_label = Label::new("double the value");
    let row = Flex::row()
        .alignment(Alignment::End)
        .with_child(checkbox.center().padding(5.0), 0.0)
        .with_child(checkbox_label, 0.0);

    let bar = LensWrap::new(ProgressBar::new(), DemoState::value);
    let slider = LensWrap::new(Slider::new(), DemoState::value);

    let button_1 = Button::new("increment ", |_ctx, data: &mut DemoState, _env| {
        data.value += 0.1
    })
    .fix_size(200.0, 100.0)
    .align_vertical(UnitPoint::CENTER);

    let button_2 = Button::new("decrement ", |_ctx, data: &mut DemoState, _env| {
        data.value -= 0.1
    });

    Flex::column()
        .with_child(Padding::new(5.0, bar), 1.0)
        .with_child(Padding::new(5.0, slider), 1.0)
        .with_child(Padding::new(5.0, label), 1.0)
        .with_child(Padding::new(5.0, row), 1.0)
        .with_child(Padding::new(5.0, Align::right(button_1)), 0.0)
        .with_child(Padding::new(5.0, button_2), 1.0)
}

fn main() {
    let window = WindowDesc::new(build_widget)
        .title(LocalizedString::new("slider-demo-window-title").with_placeholder("Sliding along"));
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(DemoState {
            value: 0.7f64,
            double: false,
        })
        .expect("launch failed");
}
