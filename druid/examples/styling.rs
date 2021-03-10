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

use druid::widget::{Button, ButtonStyle, Flex};
use druid::{AppLauncher, Data, Lens, UnitPoint, Widget, WidgetExt, WindowDesc};

#[derive(Clone, Data, Lens)]
struct AppState {}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title("Styling")
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = AppState {};

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    // We'll base our custom style on the default ButtonStyle with a couple overrides
    let normal_button_style = ButtonStyle {
        border_radius: (0.).into(),
        border_width: (0.).into(),
        background_is_gradient: false.into(),
        background_color_a: druid::theme::PRIMARY_DARK.into(),
        ..Default::default()
    };

    // Then hot and active are just mild modifications to our normal style
    let hot_button_style = ButtonStyle {
        background_color_a: druid::theme::PRIMARY_LIGHT.into(),
        ..normal_button_style.clone()
    };

    let active_button_style = ButtonStyle {
        background_color_a: druid::theme::BUTTON_DARK.into(),
        ..hot_button_style.clone()
    };

    // A regular button with default styling
    let button = Button::new("Heyo");

    // A button with all three style states customized
    let styled_button = Button::new("Heyo")
        .with_style_normal(normal_button_style)
        .with_style_hot(hot_button_style)
        .with_style_active(active_button_style);

    Flex::column()
        .with_child(button)
        .with_spacer(20.)
        .with_child(styled_button)
        .align_vertical(UnitPoint::CENTER)
}
