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

use druid::piet::Color;
use druid::widget::{Button, Flex, Label, WidgetExt};
use druid::{theme, AppLauncher, LocalizedString, PlatformError, Widget, WindowDesc};

fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("hello-demo-window-title").with_placeholder("Hello World!"));
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)?;

    Ok(())
}

fn ui_builder() -> impl Widget<u32> {
    let text =
        LocalizedString::new("hello-counter").with_arg("count", |data: &u32, _env| (*data).into());

    let label = Label::new(text.clone())
        .padding(5.0)
        .border(theme::LABEL_COLOR, theme::SCROLL_BAR_WIDTH);

    let black_label = Label::new(text)
        .padding(5.0)
        .border(theme::LABEL_COLOR, 1.0)
        .env_scope(|env, _| {
            env.set(theme::FONT_NAME, "Wingdings");
            env.set(theme::LABEL_COLOR, Color::rgb8(0, 0, 0));
        });

    let button = Button::new("increment", |_ctx, data, _env| *data += 1);

    Flex::column()
        .with_child(label.center(), 1.0)
        .with_child(black_label.center(), 1.0)
        .with_child(button.padding(5.0), 1.0)
        .background(theme::SELECTION_COLOR)
}
