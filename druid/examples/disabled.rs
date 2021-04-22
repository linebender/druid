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

//! An example showing the effect of the disabled state on focus-chain and the widgets.
//! When the disabled checkbox is clicked only the widgets not marked as disabled should
//! respond. Pressing Tab should only focus widgets not marked as disabled. If a widget
//! is focused while getting disabled it should resign the focus.

use druid::widget::{
    Button, Checkbox, CrossAxisAlignment, Flex, Label, Slider, Stepper, Switch, TextBox,
};
use druid::{AppLauncher, Data, Lens, LocalizedString, UnitPoint, Widget, WidgetExt, WindowDesc};

#[derive(Clone, Data, Lens)]
struct AppData {
    option: bool,
    text: String,
    value: f64,

    disabled: bool,
}

fn named_child(name: &str, widget: impl Widget<AppData> + 'static) -> impl Widget<AppData> {
    Flex::row()
        .with_child(Label::new(name))
        .with_default_spacer()
        .with_child(widget)
}

fn main_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_child(named_child("text:", TextBox::new().lens(AppData::text)))
        .with_default_spacer()
        .with_child(
            named_child("text (disabled):", TextBox::new().lens(AppData::text))
                .disabled_if(|data, _| data.disabled),
        )
        .with_default_spacer()
        .with_child(named_child("text:", TextBox::new().lens(AppData::text)))
        .with_default_spacer()
        .with_child(
            named_child("text (disabled):", TextBox::new().lens(AppData::text))
                .disabled_if(|data, _| data.disabled),
        )
        .with_default_spacer()
        .with_default_spacer()
        .with_child(
            named_child(
                "value (disabled):",
                Slider::new().with_range(0.0, 10.0).lens(AppData::value),
            )
            .disabled_if(|data, _| data.disabled),
        )
        .with_default_spacer()
        .with_child(
            named_child(
                "value (disabled):",
                Stepper::new()
                    .with_range(0.0, 10.0)
                    .with_step(0.5)
                    .lens(AppData::value),
            )
            .disabled_if(|data, _| data.disabled),
        )
        .with_default_spacer()
        .with_child(
            named_child(
                "option (disabled):",
                Checkbox::new("option").lens(AppData::option),
            )
            .disabled_if(|data, _| data.disabled),
        )
        .with_default_spacer()
        .with_child(
            named_child("option (disabled):", Switch::new().lens(AppData::option))
                .disabled_if(|data, _| data.disabled),
        )
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("-")
                        .on_click(|_, data: &mut f64, _| *data -= 1.0)
                        .disabled_if(|data, _| *data < 1.0),
                )
                .with_default_spacer()
                .with_child(Label::dynamic(|data: &f64, _| data.to_string()))
                .with_default_spacer()
                .with_child(
                    Button::new("+")
                        .on_click(|_, data: &mut f64, _| *data += 1.0)
                        .disabled_if(|data, _| *data > 9.0),
                )
                .lens(AppData::value)
                .disabled_if(|data: &AppData, _| data.disabled),
        )
        .with_default_spacer()
        .with_default_spacer()
        .with_default_spacer()
        .with_child(Checkbox::new("disabled").lens(AppData::disabled))
        .with_default_spacer()
        .cross_axis_alignment(CrossAxisAlignment::End)
        .align_horizontal(UnitPoint::CENTER)
}

pub fn main() {
    let window = WindowDesc::new(main_widget()).title(
        LocalizedString::new("disabled-demo-window-title").with_placeholder("Disabled demo"),
    );
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(AppData {
            option: true,
            text: "a very important text!".to_string(),
            value: 2.0,
            disabled: false,
        })
        .expect("launch failed");
}
