// Copyright 2020 The xi-editor Authors.
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

//! Demonstrates alignment and padding of children in the flex container.

use druid::widget::{
    Alignment, Button, Checkbox, Either, Flex, Label, ProgressBar, Slider, Stepper, Switch,
    TextBox, WidgetExt,
};
use druid::{AppLauncher, Data, Env, Lens, LocalizedString, PlatformError, Widget, WindowDesc};

#[derive(Clone, Data, Lens)]
pub struct AppState {
    vertical: bool,
    pub input_text: String,
    pub enabled: bool,
    volume: f64,
}

fn make_flex(flex: Flex<AppState>) -> impl Widget<AppState> {
    flex.with_child(TextBox::new().lens(AppState::input_text).center(), 0.)
        .with_child(
            Button::new("Clear", |_ctx, data: &mut AppState, _env| {
                data.input_text.clear();
                data.enabled = false;
                data.volume = 0.0;
            })
            .center(),
            0.,
        )
        .with_child(
            Label::new(|data: &AppState, _: &Env| data.input_text.clone()).center(),
            0.,
        )
        .with_child(Checkbox::new().lens(AppState::enabled).center(), 0.)
        .with_child(Slider::new().lens(AppState::volume).center(), 0.)
        .with_child(ProgressBar::new().lens(AppState::volume).center(), 0.)
        .with_child(
            Stepper::new()
                .min(0.0)
                .max(1.0)
                .step(0.1)
                .lens(AppState::volume),
            0.0,
        )
        .with_child(Switch::new().lens(AppState::enabled), 0.)
        .padding((0., 5.0))
}

fn make_ui() -> impl Widget<AppState> {
    let horiz = Flex::column()
        .with_child(Label::new("top aligned").padding((0., 10., 0., 0.)), 0.)
        .with_child(make_flex(Flex::row().alignment(Alignment::Start)), 0.)
        .with_child(Label::new("center aligned").padding((0., 10., 0., 0.)), 0.)
        .with_child(
            make_flex(
                Flex::row()
                    .alignment(Alignment::Center)
                    .child_spacing(druid::theme::CONTROL_SPACING_HORIZ),
            ),
            0.,
        )
        .with_child(Label::new("bottom aligned").padding((0., 10., 0., 0.)), 0.)
        .with_child(
            make_flex(Flex::row().alignment(Alignment::End).child_spacing(20.)),
            0.,
        )
        .center();

    let vert = Flex::row()
        .child_spacing(10.)
        .with_child(make_flex(Flex::column().alignment(Alignment::Start)), 0.)
        .with_child(
            make_flex(
                Flex::column()
                    .alignment(Alignment::Center)
                    .child_spacing(8.),
            ),
            0.,
        )
        .with_child(
            make_flex(Flex::column().alignment(Alignment::End).child_spacing(20.)),
            0.,
        )
        .center();

    let either = Either::new(|data, _| data.vertical, vert, horiz);

    Flex::column()
        .child_spacing(20.)
        .with_child(
            Label::new(|data: &AppState, _: &Env| {
                if data.vertical {
                    "Vertical".into()
                } else {
                    "Horiziontal".into()
                }
            })
            .center(),
            0.0,
        )
        .with_child(either, 0.0)
        .with_child(
            Button::new("Toggle Axis", |_, data: &mut AppState, _| {
                data.vertical = !data.vertical
            })
            .padding(10.)
            .center(),
            0.0,
        )
        .center()
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(make_ui)
        .window_size((700., 500.00))
        .title(LocalizedString::new("Container Alignment"));

    let data = AppState {
        input_text: "hello".into(),
        vertical: false,
        enabled: false,
        volume: 0.0,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)?;
    Ok(())
}
