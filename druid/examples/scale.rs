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

use druid::widget::{
    Button, Checkbox, Container, EnvScope, Flex, Label, Padding, ProgressBar, RadioGroup, Slider,
    Spinner, Stepper, Switch, TextBox, WidgetExt,
};
use druid::{theme, AppLauncher, Color, Data, Env, Lens, LocalizedString, Widget, WindowDesc};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("scale-demo-window-title").with_placeholder("Scaling demo"));
    let data = State {
        scale: 4.0,
        value: 4.0,
        text: "TextBox".into(),
        checkbox: false,
        radio: RadioEnum::One,
        progress: 0.5,
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

#[derive(Clone, Data, Lens)]
struct State {
    scale: f64,
    value: f64,
    text: String,
    checkbox: bool,
    radio: RadioEnum,
    progress: f64,
}

#[derive(Clone, Data, PartialEq)]
enum RadioEnum {
    One,
    Two,
}

fn ui_builder() -> impl Widget<State> {
    Flex::column()
        .with_child(EnvScope::new(
            |env: &mut Env, _data: &State| env.set(theme::SCALE, 3.0),
            Slider::new().with_range(0.8, 5.0).lens(State::scale),
        ))
        .with_child(EnvScope::new(
            |env: &mut Env, data: &State| env.set(theme::SCALE, data.scale),
            Flex::column()
                .with_child(
                    Label::new(|data: &f64, _env: &_| format!("Dynamic Scale: {}", data))
                        .fix_width(200.0)
                        .lens(State::scale),
                )
                .with_child(
                    Flex::column().with_child(
                        Flex::row()
                            .with_child(
                                Flex::column()
                                    .with_child(
                                        Container::new(Label::new("Unpadded").fix_width(80.0))
                                            .background(Color::rgb8(200, 150, 100)),
                                    )
                                    .with_child(Padding::new(
                                        (0.0, 5.0, 2.5, 0.0),
                                        Container::new(Label::new("Padded").fix_width(80.0))
                                            .background(Color::rgb8(255, 0, 0)),
                                    )),
                            )
                            .with_child(
                                Flex::column()
                                    .with_child(
                                        Container::new(Label::new("Unpadded").fix_width(80.0))
                                            .background(Color::rgb8(200, 150, 100)),
                                    )
                                    .with_child(
                                        Container::new(Label::new("Unpadded").fix_width(80.0))
                                            .background(Color::rgb8(200, 150, 100)),
                                    ),
                            ),
                    ),
                )
                .with_child(
                    Flex::row()
                        .with_child(TextBox::new().lens(State::text))
                        .with_child(Button::new("Button"))
                        .with_child(Checkbox::new("CheckBox").lens(State::checkbox)),
                )
                .with_child(
                    Flex::row()
                        .with_child(Spinner::new())
                        .with_child(Stepper::new().lens(State::scale))
                        .with_child(Switch::new().lens(State::checkbox)),
                )
                .with_child(Slider::new().with_range(0.8, 5.0).lens(State::scale))
                .with_child(
                    RadioGroup::new(vec![
                        ("Radio 1", RadioEnum::One),
                        ("Radio 2", RadioEnum::Two),
                    ])
                    .lens(State::radio),
                )
                .with_child(ProgressBar::new().lens(State::progress)),
        ))
        .with_child(Label::new("Static Scale (3.0, 1.5)"))
        .with_child(
            Flex::row()
                .with_child(EnvScope::new(
                    |env: &mut Env, _data: &State| env.set(theme::SCALE, 3.0),
                    TextBox::new().lens(State::text),
                ))
                .with_child(EnvScope::new(
                    |env: &mut Env, _data: &State| env.set(theme::SCALE, 1.5),
                    Flex::column()
                        .with_child(TextBox::new().lens(State::text))
                        .with_child(TextBox::new().lens(State::text)),
                )),
        )
        .with_child(Label::new("Unscaled"))
        .with_child(
            Flex::row()
                .with_child(TextBox::new().lens(State::text))
                .with_child(Button::new("Button"))
                .with_child(Checkbox::new("CheckBox").lens(State::checkbox)),
        )
}
