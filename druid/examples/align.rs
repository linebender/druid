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

//! Demonstrates alignment of children in the flex container.

use druid::widget::{
    Alignment, Button, Checkbox, Flex, Label, ProgressBar, Slider, Stepper, Switch, TextBox,
    WidgetExt,
};
use druid::{
    AppLauncher, Color, Data, Env, Lens, LocalizedString, PlatformError, Widget, WindowDesc,
};

#[derive(Clone, Data, Lens)]
pub struct AppState {
    pub input_text: String,
    pub enabled: bool,
    volume: f64,
}

fn make_widget_row(alignment: Alignment) -> impl Widget<AppState> {
    Flex::row()
        .alignment(alignment)
        .with_child(TextBox::new().lens(AppState::input_text).center(), 0.)
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
        .background(Color::rgba8(0, 0, 0xFF, 0x40))
        .padding((0., 5.0))
}

fn make_ui() -> impl Widget<AppState> {
    Flex::column()
        .with_child(Label::new("top aligned").padding((0., 10., 0., 0.)), 0.)
        .with_child(make_widget_row(Alignment::Start), 0.)
        .with_child(Label::new("center aligned").padding((0., 10., 0., 0.)), 0.)
        .with_child(make_widget_row(Alignment::Center), 0.)
        .with_child(Label::new("bottom aligned").padding((0., 10., 0., 0.)), 0.)
        .with_child(make_widget_row(Alignment::End), 0.)
        .center()
}

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(make_ui)
        .window_size((550., 320.00))
        .title(LocalizedString::new("Container Alignment"));

    let data = AppState {
        input_text: "hello".into(),
        enabled: false,
        volume: 0.0,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)?;
    Ok(())
}
