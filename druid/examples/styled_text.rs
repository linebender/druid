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

use druid::widget::{Alignment, Flex, Label, Parse, Stepper, TextBox, WidgetExt};
use druid::{
    theme, AppLauncher, Data, Lens, LensExt, LensWrap, LocalizedString, PlatformError, Widget,
    WindowDesc,
};

#[derive(Clone, Lens, Data)]
struct AppData {
    text: String,
    size: f64,
}
fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder).title(
        LocalizedString::new("styled-text-demo-window-title").with_placeholder("Type Styler"),
    );
    let data = AppData {
        text: "This is what text looks like".to_string(),
        size: 24.0,
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)?;

    Ok(())
}

fn ui_builder() -> impl Widget<AppData> {
    // This is druid's default text style.
    // It's set by theme::LABEL_COLOR and theme::TEXT_SIZE_NORMAL
    let label =
        Label::new(|data: &String, _env: &_| format!("Default: {}", data)).lens(AppData::text);

    // The text_color and text_size builder methods can override the defaults
    // with either a theme key (like we're doing here), or a concrete value
    // (Color and f64, respectively).
    let styled_label =
        Label::new(|data: &AppData, _env: &_| format!("Size {:.1}: {}", data.size, data.text))
            .text_color(theme::PRIMARY_LIGHT)
            .text_size(theme::TEXT_SIZE_LARGE)
            .env_scope(|env: &mut druid::Env, data: &AppData| {
                env.set(theme::TEXT_SIZE_LARGE, data.size)
            });

    let stepper = Stepper::new()
        .max(100.0)
        .min(0.0)
        .step(1.0)
        .wrap(false)
        .lens(AppData::size);

    let stepper_textbox = LensWrap::new(
        Parse::new(TextBox::new()),
        AppData::size.map(|x| Some(*x), |x, y| *x = y.unwrap_or(24.0)),
    );

    let stepper_row = Flex::row()
        .alignment(Alignment::Center)
        .with_child(stepper_textbox, 0.0)
        .with_child(stepper, 0.0);

    let input = TextBox::new().lens(AppData::text);

    Flex::column()
        .with_child(label.center(), 1.0)
        .with_child(styled_label.center(), 1.0)
        .with_child(stepper_row.center(), 1.0)
        .with_child(input.padding(5.0).center(), 1.0)
}
