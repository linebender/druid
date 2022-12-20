// Copyright 2019 The Druid Authors.
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

//! This is an example of how to translate and localize a druid application.
//! It uses the fluent (.ftl) files in the asset directory for defining defining messages.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::{prelude::*, Slider};
use druid::widget::{Flex, Label};
use druid::{AppLauncher, Data, Lens, LocalizedString, UnitPoint, WidgetExt, WindowDesc};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const SLIDER_WIDTH: f64 = 200.0;

#[derive(Clone, Data, Lens)]
struct BananaState {
    count: f64,
}

pub fn main() {
    let main_window = WindowDesc::new(build_root_widget())
        .title(LocalizedString::new("banana-title"))
        .window_size((400.0, 400.0));

    let initial_state: BananaState = BananaState { count: 1f64 };

    // start the application, referencing the translation files in /assets.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .localization_resources(vec!["banana.ftl".into()], "assets".into())
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<BananaState> {
    // create a label with a static translation
    let title = Label::new(LocalizedString::new("banana-title")).with_text_size(28.0);

    // create a label that uses a translation with dynamic arguments
    let banana_label = Label::new(|data: &BananaState, env: &Env| {
        let mut s = LocalizedString::<BananaState>::new("bananas")
            .with_arg("count", |d, _e| d.count.into());
        s.resolve(data, env);

        s.localized_str()
    })
    .with_text_size(32.0);

    // control the banana count
    let slider = Slider::new()
        .with_range(0.0, 3.0)
        .with_step(1.0)
        .fix_width(SLIDER_WIDTH)
        .lens(BananaState::count);

    Flex::column()
        .with_child(title)
        .with_spacer(VERTICAL_WIDGET_SPACING * 2.0)
        .with_child(banana_label)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(slider)
        .align_vertical(UnitPoint::CENTER)
}
